#!/usr/bin/env python3
"""Validate that a stable promotion is backed by a complete RC release."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[3]
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
RC_TAG_RE = re.compile(r"^v(?P<version>[0-9]+\.[0-9]+\.[0-9]+)-rc\.(?P<number>[0-9]+)$")
ED25519_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")


def main() -> int:
    args = parse_args()
    errors: list[str] = []

    if not VERSION_RE.fullmatch(args.version):
        errors.append("version must be MAJOR.MINOR.PATCH")
    rc_match = RC_TAG_RE.fullmatch(args.rc_tag)
    if not rc_match:
        errors.append("rc-tag must be vMAJOR.MINOR.PATCH-rc.N")
    elif rc_match.group("version") != args.version:
        errors.append(f"rc-tag {args.rc_tag} does not belong to version {args.version}")

    if errors:
        print_errors(errors)
        return 1

    rc_number = rc_match.group("number") if rc_match else ""
    rc_version = f"{args.version}-rc.{rc_number}"
    contract = load_release_contract(args.contract)
    expected = expected_rc_assets(contract, args.version, rc_number)

    release = load_release(args)
    errors.extend(validate_release_metadata(release, args.rc_tag, rc_version, expected.required_assets))
    if errors:
        print_errors(errors)
        return 1

    with local_asset_dir(args, expected.required_assets) as asset_dir:
        errors.extend(validate_downloaded_assets(asset_dir, expected, args.checksum_public_key))

    if errors:
        print_errors(errors)
        return 1

    print(f"Promoted RC release {args.rc_tag} is complete.")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True, help="Stable version, e.g. 19.1.0")
    parser.add_argument("--rc-tag", required=True, help="Promoted RC tag, e.g. v19.1.0-rc.2")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""), help="GitHub repo owner/name")
    parser.add_argument(
        "--contract",
        type=Path,
        default=REPO_ROOT / "release_contract.toml",
        help="Release contract TOML path",
    )
    parser.add_argument(
        "--release-json",
        type=Path,
        help="Fixture JSON from `gh release view --json tagName,isPrerelease,body,assets`",
    )
    parser.add_argument("--asset-dir", type=Path, help="Fixture directory containing downloaded RC assets")
    parser.add_argument(
        "--checksum-public-key",
        help="Optional base64 Ed25519 public key used to verify the checksum signature",
    )
    return parser.parse_args()


def load_release_contract(path: Path) -> dict[str, Any]:
    try:
        import tomllib  # type: ignore[import-not-found]

        return tomllib.loads(path.read_text(encoding="utf-8"))
    except ModuleNotFoundError:
        return parse_minimal_release_contract(path.read_text(encoding="utf-8"))


def parse_minimal_release_contract(text: str) -> dict[str, Any]:
    app_name = require_match(r'^app_name\s*=\s*"([^"]+)"', text, "app_name")
    targets_match = require_match(r"(?ms)^targets\s*=\s*\[(.*?)\]", text, "targets")
    targets = re.findall(r'"([^"]+)"', targets_match)
    templates_match = require_match(r"(?ms)^\[templates\]\s*(.*?)(?:^\[|\Z)", text, "templates")
    templates = dict(re.findall(r'(?m)^([A-Za-z0-9_]+)\s*=\s*"([^"]+)"', templates_match))
    return {"app_name": app_name, "targets": targets, "templates": templates}


def require_match(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise SystemExit(f"release contract is missing {label}")
    return match.group(1)


class ExpectedAssets:
    def __init__(self, zips: list[str], checksum: str, signature: str) -> None:
        self.zips = zips
        self.checksum = checksum
        self.signature = signature
        self.required_assets = zips + [checksum, signature]


def expected_rc_assets(contract: dict[str, Any], version: str, rc_number: str) -> ExpectedAssets:
    app_name = contract["app_name"]
    templates = contract["templates"]
    zips = [
        apply_template(templates["rc_asset"], app_name, version, rc_number, platform_for_target(target), arch_for_target(target))
        for target in contract["targets"]
    ]
    checksum = apply_template(templates["rc_checksums"], app_name, version, rc_number, "", "")
    signature = apply_template(templates["rc_checksums_sig"], app_name, version, rc_number, "", "")
    return ExpectedAssets(sorted(zips), checksum, signature)


def apply_template(
    template: str,
    app_name: str,
    version: str,
    rc_number: str,
    platform: str,
    arch: str,
) -> str:
    return (
        template.replace("{APP_NAME}", app_name)
        .replace("{version}", version)
        .replace("{rc_number}", rc_number)
        .replace("{platform}", platform)
        .replace("{arch}", arch)
    )


def platform_for_target(target: str) -> str:
    if "-pc-windows-" in target:
        return "windows"
    if target.endswith("-apple-darwin"):
        return "macos"
    if "-unknown-linux-" in target:
        return "linux"
    raise SystemExit(f"unsupported release target triple: {target}")


def arch_for_target(target: str) -> str:
    return target.split("-", 1)[0]


def load_release(args: argparse.Namespace) -> dict[str, Any]:
    if args.release_json:
        return json.loads(args.release_json.read_text(encoding="utf-8"))
    if not args.repo:
        raise SystemExit("Missing --repo or GITHUB_REPOSITORY for live GitHub release validation")
    output = run(
        [
            "gh",
            "release",
            "view",
            args.rc_tag,
            "--repo",
            args.repo,
            "--json",
            "tagName,isPrerelease,body,assets",
        ],
        "view promoted RC GitHub release",
    )
    return json.loads(output)


def validate_release_metadata(
    release: dict[str, Any],
    rc_tag: str,
    rc_version: str,
    required_assets: list[str],
) -> list[str]:
    errors: list[str] = []
    tag_name = release.get("tagName")
    if tag_name and tag_name != rc_tag:
        errors.append(f"RC release tagName is {tag_name}, expected {rc_tag}")
    if release.get("isPrerelease") is not True:
        errors.append(f"RC release {rc_tag} must be marked as a prerelease")

    body = str(release.get("body") or "").strip()
    if not body:
        errors.append(f"RC release {rc_tag} must have a non-empty release log body")
    elif f"Wavecrate {rc_version}" not in body:
        errors.append(f"RC release body must be release-bound to Wavecrate {rc_version}")

    asset_names = release_asset_names(release)
    missing = [name for name in required_assets if name not in asset_names]
    if missing:
        errors.append("RC release is missing required assets: " + ", ".join(missing))
    return errors


def release_asset_names(release: dict[str, Any]) -> set[str]:
    assets = release.get("assets") or []
    if isinstance(assets, dict):
        assets = assets.get("nodes") or []
    names = set()
    for asset in assets:
        if isinstance(asset, dict) and asset.get("name"):
            names.add(str(asset["name"]))
    return names


class local_asset_dir:
    def __init__(self, args: argparse.Namespace, required_assets: list[str]) -> None:
        self.args = args
        self.required_assets = required_assets
        self.temp: tempfile.TemporaryDirectory[str] | None = None

    def __enter__(self) -> Path:
        if self.args.asset_dir:
            return self.args.asset_dir
        if not self.args.repo:
            raise SystemExit("Missing --repo or GITHUB_REPOSITORY for live GitHub asset downloads")
        self.temp = tempfile.TemporaryDirectory(prefix="wavecrate-promoted-rc-")
        asset_dir = Path(self.temp.name)
        patterns = []
        for asset in self.required_assets:
            patterns.extend(["--pattern", asset])
        run(
            [
                "gh",
                "release",
                "download",
                self.args.rc_tag,
                "--repo",
                self.args.repo,
                "--dir",
                str(asset_dir),
                "--clobber",
                *patterns,
            ],
            "download promoted RC release assets",
        )
        return asset_dir

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        if self.temp:
            self.temp.cleanup()


def validate_downloaded_assets(
    asset_dir: Path,
    expected: ExpectedAssets,
    checksum_public_key: str | None,
) -> list[str]:
    errors: list[str] = []
    for asset in expected.required_assets:
        path = asset_dir / asset
        if not path.is_file():
            errors.append(f"Downloaded RC asset is missing: {asset}")
        elif path.stat().st_size == 0:
            errors.append(f"Downloaded RC asset is empty: {asset}")

    checksum_path = asset_dir / expected.checksum
    if checksum_path.is_file():
        errors.extend(validate_checksum_file(checksum_path, asset_dir, expected.zips))

    signature_path = asset_dir / expected.signature
    if checksum_public_key and checksum_path.is_file() and signature_path.is_file():
        errors.extend(verify_checksum_signature(checksum_path, signature_path, checksum_public_key))
    return errors


def validate_checksum_file(checksum_path: Path, asset_dir: Path, expected_zips: list[str]) -> list[str]:
    errors: list[str] = []
    entries: dict[str, str] = {}
    for line_number, line in enumerate(checksum_path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = line.strip()
        if not stripped:
            continue
        parts = stripped.split()
        if len(parts) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", parts[0]):
            errors.append(f"{checksum_path.name}:{line_number} is not a SHA-256 checksum entry")
            continue
        entries[parts[1]] = parts[0].lower()

    for zip_name in expected_zips:
        expected_hash = entries.get(zip_name)
        if not expected_hash:
            errors.append(f"Checksum file is missing entry for {zip_name}")
            continue
        zip_path = asset_dir / zip_name
        if not zip_path.is_file():
            continue
        actual_hash = sha256(zip_path)
        if actual_hash != expected_hash:
            errors.append(
                f"Checksum mismatch for {zip_name}: checksum file has {expected_hash}, asset hashes to {actual_hash}"
            )
    return errors


def verify_checksum_signature(checksum_path: Path, signature_path: Path, checksum_public_key: str) -> list[str]:
    if not shutil.which("openssl"):
        return ["openssl is required to verify the RC checksum signature"]
    try:
        public_key = base64.b64decode(checksum_public_key, validate=True)
    except ValueError as error:
        return [f"invalid base64 checksum public key: {error}"]
    if len(public_key) != 32:
        return ["checksum public key must decode to a 32-byte Ed25519 public key"]

    with tempfile.TemporaryDirectory(prefix="wavecrate-rc-signature-") as temp:
        pub_der = Path(temp) / "checksum-public-key.der"
        sig_bin = Path(temp) / "checksums.sig"
        pub_der.write_bytes(ED25519_SPKI_PREFIX + public_key)
        try:
            sig_bin.write_bytes(base64.b64decode(signature_path.read_text(encoding="utf-8"), validate=True))
        except ValueError as error:
            return [f"checksum signature is not valid base64: {error}"]
        completed = subprocess.run(
            [
                "openssl",
                "pkeyutl",
                "-verify",
                "-pubin",
                "-inkey",
                str(pub_der),
                "-keyform",
                "DER",
                "-rawin",
                "-in",
                str(checksum_path),
                "-sigfile",
                str(sig_bin),
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    if completed.returncode != 0:
        return ["RC checksum signature verification failed: " + completed.stderr.strip()]
    return []


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(command: list[str], label: str) -> str:
    completed = subprocess.run(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(f"Failed to {label}:\n{completed.stderr.strip()}")
    return completed.stdout


def print_errors(errors: list[str]) -> None:
    print("Promoted RC release validation failed:", file=sys.stderr)
    for error in errors:
        print(f" - {error}", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
