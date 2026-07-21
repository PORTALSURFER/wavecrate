#!/usr/bin/env python3
"""Verify published Wavecrate release assets after public upload."""

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
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[3]
ED25519_SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")


@dataclass(frozen=True)
class ExpectedZip:
    name: str
    target: str
    platform: str
    arch: str


@dataclass(frozen=True)
class ExpectedRelease:
    zips: list[ExpectedZip]
    checksum: str
    signature: str

    @property
    def required_assets(self) -> list[str]:
        return [zip_asset.name for zip_asset in self.zips] + [self.checksum, self.signature]


@dataclass(frozen=True)
class ArchiveLayout:
    root_dir: str
    files: list[str]
    optional_dirs: list[str]
    platform_files: dict[str, list[str]]

    def expected_files_for(self, platform: str) -> list[str]:
        explicit = self.platform_files.get(platform)
        if explicit is not None:
            return explicit
        if platform == "windows":
            return [path for path in self.files if path.endswith(".exe")]
        return [path for path in self.files if not path.endswith(".exe")]


def main() -> int:
    args = parse_args()
    errors = validate_args(args)
    if errors:
        print_errors(errors)
        return 1

    contract = load_release_contract(args.contract)
    expected = expected_release_assets(contract, args.channel, args.version, args.target_version)
    archive_layout = archive_layout_from_contract(contract)
    release = load_release(args)
    errors.extend(validate_release_metadata(release, args, expected.required_assets))
    if errors:
        print_errors(errors)
        return 1

    with local_asset_dir(args, release, expected.required_assets) as asset_dir:
        errors.extend(validate_downloaded_assets(asset_dir, release, expected, archive_layout, args))

    if errors:
        print_errors(errors)
        return 1

    print(f"Published {args.channel} release {args.version} verified from {args.source}.")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--surface",
        default="github",
        choices=["github", "portalsurfer"],
        help="Public release surface to verify",
    )
    parser.add_argument("--channel", required=True, choices=["nightly", "rc", "stable"])
    parser.add_argument("--version", required=True, help="Release version embedded in update-manifest.json")
    parser.add_argument("--target-version", required=True, help="Stable target version, e.g. 0.19.1")
    parser.add_argument("--commit", required=True, help="Expected release commit SHA")
    parser.add_argument("--build-date", required=True, help="Expected release build date, YYYY-MM-DD")
    parser.add_argument("--tag", help="GitHub release tag to inspect")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""), help="GitHub repo owner/name")
    parser.add_argument("--portal-catalog-url", help="PortalSurfer releases catalog URL")
    parser.add_argument("--portal-build-id", help="PortalSurfer release build_id to inspect")
    parser.add_argument(
        "--portal-download-token",
        help="Pre-issued PortalSurfer download token for fixture or diagnostic runs",
    )
    parser.add_argument("--build-number", type=int, help="Expected PortalSurfer build number")
    parser.add_argument(
        "--source",
        default="",
        help="Human-readable public surface name for diagnostics",
    )
    parser.add_argument(
        "--contract",
        type=Path,
        default=REPO_ROOT / "release_contract.toml",
        help="Release contract TOML path",
    )
    parser.add_argument(
        "--release-json",
        type=Path,
        help="Fixture JSON from `gh release view --json tagName,isPrerelease,targetCommitish,body,assets`",
    )
    parser.add_argument("--asset-dir", type=Path, help="Fixture directory containing downloaded release assets")
    parser.add_argument(
        "--checksum-public-key",
        required=True,
        help="Base64 Ed25519 public key used to verify the checksum signature",
    )
    args = parser.parse_args()
    if not args.source:
        args.source = "GitHub" if args.surface == "github" else "PortalSurfer"
    return args


def validate_args(args: argparse.Namespace) -> list[str]:
    errors: list[str] = []
    if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", args.build_date):
        errors.append("build-date must be YYYY-MM-DD")
    if not re.fullmatch(r"[0-9a-fA-F]{7,40}", args.commit):
        errors.append("commit must be a 7-40 character hexadecimal SHA")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.target_version):
        errors.append("target-version must be MAJOR.MINOR.PATCH")
    if args.channel == "stable" and args.version != args.target_version:
        errors.append("stable release version must equal target-version")
    if args.channel == "rc" and not re.fullmatch(
        re.escape(args.target_version) + r"-rc\.[0-9]+",
        args.version,
    ):
        errors.append("rc release version must be TARGET_VERSION-rc.N")
    if args.channel == "nightly" and not args.version.startswith(f"{args.target_version}-nightly."):
        errors.append("nightly release version must start with TARGET_VERSION-nightly.")
    if args.surface == "github":
        if not args.tag:
            errors.append("github verification requires --tag")
        if not args.release_json and not args.repo:
            errors.append("github live verification requires --repo or GITHUB_REPOSITORY")
    else:
        if not args.portal_build_id:
            errors.append("PortalSurfer verification requires --portal-build-id")
        if args.build_number is None:
            errors.append("PortalSurfer verification requires --build-number")
        if not args.release_json and not args.portal_catalog_url:
            errors.append("PortalSurfer live verification requires --portal-catalog-url")
    return errors


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
    archive_match = require_match(r"(?ms)^\[archive_layout\]\s*(.*?)(?:^\[|\Z)", text, "archive_layout")
    root_dir = require_match(r'(?m)^root_dir\s*=\s*"([^"]+)"', archive_match, "archive_layout.root_dir")
    files = parse_toml_string_array(archive_match, "files")
    optional_dirs = parse_toml_string_array(archive_match, "optional_dirs")
    platform_files_match = re.search(
        r"(?ms)^\[archive_layout\.platform_files\]\s*(.*?)(?:^\[|\Z)",
        text,
    )
    platform_files: dict[str, list[str]] = {}
    if platform_files_match:
        for platform in ("windows", "macos", "linux"):
            values = parse_toml_string_array(platform_files_match.group(1), platform)
            if values:
                platform_files[platform] = values
    return {
        "app_name": app_name,
        "targets": targets,
        "templates": templates,
        "archive_layout": {
            "root_dir": root_dir,
            "files": files,
            "optional_dirs": optional_dirs,
            "platform_files": platform_files,
        },
    }


def parse_toml_string_array(text: str, key: str) -> list[str]:
    match = re.search(rf"(?ms)^{re.escape(key)}\s*=\s*\[(.*?)\]", text)
    if not match:
        return []
    return re.findall(r'"([^"]+)"', match.group(1))


def require_match(pattern: str, text: str, label: str) -> str:
    match = re.search(pattern, text)
    if not match:
        raise SystemExit(f"release contract is missing {label}")
    return match.group(1)


def expected_release_assets(
    contract: dict[str, Any],
    channel: str,
    version: str,
    target_version: str,
) -> ExpectedRelease:
    app_name = contract["app_name"]
    templates = contract["templates"]
    targets = [
        (target, platform_for_target(target), arch_for_target(target))
        for target in contract["targets"]
    ]
    if channel == "nightly":
        zips = [
            ExpectedZip(
                apply_template(templates["nightly_asset"], app_name, version, "", platform, arch),
                target,
                platform,
                arch,
            )
            for target, platform, arch in targets
        ]
        checksum = templates["nightly_checksums"]
        signature = templates["nightly_checksums_sig"]
    elif channel == "rc":
        rc_number = version.rsplit("-rc.", 1)[1]
        zips = [
            ExpectedZip(
                apply_template(templates["rc_asset"], app_name, target_version, rc_number, platform, arch),
                target,
                platform,
                arch,
            )
            for target, platform, arch in targets
        ]
        checksum = apply_template(templates["rc_checksums"], app_name, target_version, rc_number, "", "")
        signature = apply_template(templates["rc_checksums_sig"], app_name, target_version, rc_number, "", "")
    else:
        zips = [
            ExpectedZip(
                apply_template(templates["stable_asset"], app_name, version, "", platform, arch),
                target,
                platform,
                arch,
            )
            for target, platform, arch in targets
        ]
        checksum = apply_template(templates["stable_checksums"], app_name, version, "", "", "")
        signature = apply_template(templates["stable_checksums_sig"], app_name, version, "", "", "")
    return ExpectedRelease(sorted(zips, key=lambda item: item.name), checksum, signature)


def archive_layout_from_contract(contract: dict[str, Any]) -> ArchiveLayout:
    app_name = contract["app_name"]
    layout = contract.get("archive_layout")
    if not isinstance(layout, dict):
        raise SystemExit("release contract is missing [archive_layout]")
    root_dir = str(layout.get("root_dir") or "").strip()
    if not root_dir:
        raise SystemExit("release contract is missing archive_layout.root_dir")
    files = layout.get("files")
    if not isinstance(files, list) or not all(isinstance(path, str) and path for path in files):
        raise SystemExit("release contract archive_layout.files must be a non-empty string array")
    optional_dirs = layout.get("optional_dirs") or []
    if not isinstance(optional_dirs, list) or not all(isinstance(path, str) for path in optional_dirs):
        raise SystemExit("release contract archive_layout.optional_dirs must be a string array")
    platform_files = layout.get("platform_files") or {}
    if not isinstance(platform_files, dict):
        raise SystemExit("release contract archive_layout.platform_files must be a table")

    expanded_platform_files: dict[str, list[str]] = {}
    for platform, paths in platform_files.items():
        if not isinstance(paths, list) or not all(isinstance(path, str) and path for path in paths):
            raise SystemExit(
                f"release contract archive_layout.platform_files.{platform} must be a non-empty string array"
            )
        expanded_platform_files[str(platform)] = [
            normalize_archive_relative_path(apply_archive_layout_template(path, app_name))
            for path in paths
        ]
    return ArchiveLayout(
        root_dir=normalize_archive_relative_path(apply_archive_layout_template(root_dir, app_name)),
        files=[
            normalize_archive_relative_path(apply_archive_layout_template(path, app_name))
            for path in files
        ],
        optional_dirs=[
            normalize_archive_relative_path(apply_archive_layout_template(path, app_name))
            for path in optional_dirs
        ],
        platform_files=expanded_platform_files,
    )


def apply_archive_layout_template(template: str, app_name: str) -> str:
    return template.replace("{APP_NAME}", app_name)


def normalize_archive_relative_path(path: str) -> str:
    return path.replace("\\", "/").strip("/")


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
        release = json.loads(args.release_json.read_text(encoding="utf-8"))
        if args.surface == "portalsurfer":
            return select_portalsurfer_release(release, args)
        return release
    if args.surface == "portalsurfer":
        catalog = fetch_json(args.portal_catalog_url)
        return select_portalsurfer_release(catalog, args)
    output = run(
        [
            "gh",
            "release",
            "view",
            str(args.tag),
            "--repo",
            args.repo,
            "--json",
            "tagName,isPrerelease,targetCommitish,body,assets",
        ],
        f"view {args.source} release",
    )
    return json.loads(output)


def select_portalsurfer_release(catalog_or_release: dict[str, Any], args: argparse.Namespace) -> dict[str, Any]:
    if "releases" not in catalog_or_release:
        return catalog_or_release
    releases = catalog_or_release.get("releases") or []
    release = next(
        (item for item in releases if isinstance(item, dict) and item.get("build_id") == args.portal_build_id),
        None,
    )
    if release is None:
        raise SystemExit(f"PortalSurfer catalog does not list {args.portal_build_id}")
    return release


def validate_release_metadata(
    release: dict[str, Any],
    args: argparse.Namespace,
    required_assets: list[str],
) -> list[str]:
    errors: list[str] = []
    if args.surface == "portalsurfer":
        if release.get("build_id") != args.portal_build_id:
            errors.append(
                f"{args.source} release build_id is {release.get('build_id')}, expected {args.portal_build_id}"
            )
        if release.get("build_number") != args.build_number:
            errors.append(
                f"{args.source} release build_number is {release.get('build_number')}, expected {args.build_number}"
            )
        if release.get("version") != args.version:
            errors.append(f"{args.source} release version is {release.get('version')}, expected {args.version}")
    else:
        expected_prerelease = args.channel in {"nightly", "rc"}
        if release.get("isPrerelease") is not expected_prerelease:
            errors.append(f"{args.source} {args.channel} release prerelease flag is incorrect")

        target_commitish = str(release.get("targetCommitish") or "").strip()
        if target_commitish and target_commitish != args.commit:
            errors.append(
                f"{args.source} release targetCommitish is {target_commitish}, expected {args.commit}"
            )

    tag_name = release.get("tagName")
    if args.tag and tag_name and tag_name != args.tag:
        errors.append(f"{args.source} release tagName is {tag_name}, expected {args.tag}")

    body = release_body(release, args)
    if not body:
        release_label = args.tag or args.portal_build_id or args.version
        errors.append(f"{args.source} release {release_label} must have a non-empty release log body")
    elif f"Wavecrate {args.version}" not in body:
        errors.append(f"{args.source} release body must be release-bound to Wavecrate {args.version}")

    asset_names = release_asset_names(release)
    missing = [name for name in required_assets if name not in asset_names]
    if missing:
        errors.append(f"{args.source} release is missing required assets: " + ", ".join(missing))
    return errors


def release_asset_names(release: dict[str, Any]) -> set[str]:
    assets = release_file_entries(release)
    return {
        str(asset["name"])
        for asset in assets
        if isinstance(asset, dict) and asset.get("name")
    }


def release_file_entries(release: dict[str, Any]) -> list[dict[str, Any]]:
    assets = release.get("assets")
    if isinstance(assets, dict):
        assets = assets.get("nodes") or []
    if not assets:
        assets = release.get("files") or []
    return [asset for asset in assets if isinstance(asset, dict)]


def release_body(release: dict[str, Any], args: argparse.Namespace) -> str:
    body = str(release.get("body") or "").strip()
    if body or args.surface != "portalsurfer":
        return body
    changelog = release.get("changelog") or {}
    body = str(changelog.get("body") or "").strip()
    if body:
        return body
    changelog_url = str(changelog.get("url") or "")
    if not changelog_url:
        return ""
    response = fetch_json(resolve_portalsurfer_url(args, changelog_url))
    return str((response.get("changelog") or {}).get("body") or "").strip()


class local_asset_dir:
    def __init__(self, args: argparse.Namespace, release: dict[str, Any], required_assets: list[str]) -> None:
        self.args = args
        self.release = release
        self.required_assets = required_assets
        self.temp: tempfile.TemporaryDirectory[str] | None = None

    def __enter__(self) -> Path:
        if self.args.asset_dir:
            return self.args.asset_dir
        self.temp = tempfile.TemporaryDirectory(prefix="wavecrate-published-release-")
        asset_dir = Path(self.temp.name)
        if self.args.surface == "portalsurfer":
            self.download_portalsurfer_assets(asset_dir)
            return asset_dir
        patterns = []
        for asset in self.required_assets:
            patterns.extend(["--pattern", asset])
        run(
            [
                "gh",
                "release",
                "download",
                str(self.args.tag),
                "--repo",
                self.args.repo,
                "--dir",
                str(asset_dir),
                "--clobber",
                *patterns,
            ],
            f"download {self.args.source} release assets",
        )
        return asset_dir

    def download_portalsurfer_assets(self, asset_dir: Path) -> None:
        entries = {str(item.get("name")): item for item in release_file_entries(self.release) if item.get("name")}
        download_token = self.args.portal_download_token or fetch_portalsurfer_download_token(self.args)
        for asset in self.required_assets:
            entry = entries.get(asset)
            if not entry:
                continue
            url = str(entry.get("url") or entry.get("download_url") or entry.get("href") or "")
            if not url:
                raise SystemExit(f"PortalSurfer catalog entry for {asset} does not include a download URL")
            download_url(
                portalsurfer_download_url(resolve_portalsurfer_url(self.args, url), download_token),
                asset_dir / asset,
            )

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        if self.temp:
            self.temp.cleanup()


def validate_downloaded_assets(
    asset_dir: Path,
    release: dict[str, Any],
    expected: ExpectedRelease,
    archive_layout: ArchiveLayout,
    args: argparse.Namespace,
) -> list[str]:
    errors: list[str] = []
    for asset in expected.required_assets:
        path = asset_dir / asset
        if not path.is_file():
            errors.append(f"Downloaded {args.source} asset is missing: {asset}")
        elif path.stat().st_size == 0:
            errors.append(f"Downloaded {args.source} asset is empty: {asset}")

    checksum_path = asset_dir / expected.checksum
    if checksum_path.is_file():
        errors.extend(validate_checksum_file(checksum_path, asset_dir, expected.zips))

    signature_path = asset_dir / expected.signature
    if checksum_path.is_file() and signature_path.is_file():
        errors.extend(verify_checksum_signature(checksum_path, signature_path, args.checksum_public_key))

    if args.surface == "portalsurfer":
        errors.extend(validate_portalsurfer_catalog_hashes(asset_dir, release, expected.required_assets))

    for zip_asset in expected.zips:
        zip_path = asset_dir / zip_asset.name
        if zip_path.is_file():
            errors.extend(validate_zip_manifest(zip_path, zip_asset, archive_layout, args))
    return errors


def validate_portalsurfer_catalog_hashes(
    asset_dir: Path,
    release: dict[str, Any],
    required_assets: list[str],
) -> list[str]:
    errors: list[str] = []
    entries = {str(item.get("name")): item for item in release_file_entries(release) if item.get("name")}
    for asset in required_assets:
        entry = entries.get(asset) or {}
        expected_hash = str(entry.get("sha256") or "").lower()
        if not expected_hash:
            errors.append(f"PortalSurfer catalog entry for {asset} is missing sha256")
            continue
        path = asset_dir / asset
        if path.is_file():
            actual_hash = sha256(path)
            if actual_hash != expected_hash:
                errors.append(
                    f"PortalSurfer catalog sha256 mismatch for {asset}: catalog has {expected_hash}, asset hashes to {actual_hash}"
                )
    return errors


def validate_checksum_file(
    checksum_path: Path,
    asset_dir: Path,
    expected_zips: list[ExpectedZip],
) -> list[str]:
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

    for zip_asset in expected_zips:
        expected_hash = entries.get(zip_asset.name)
        if not expected_hash:
            errors.append(f"Checksum file is missing entry for {zip_asset.name}")
            continue
        zip_path = asset_dir / zip_asset.name
        if not zip_path.is_file():
            continue
        actual_hash = sha256(zip_path)
        if actual_hash != expected_hash:
            errors.append(
                f"Checksum mismatch for {zip_asset.name}: checksum file has {expected_hash}, asset hashes to {actual_hash}"
            )
    return errors


def verify_checksum_signature(checksum_path: Path, signature_path: Path, checksum_public_key: str) -> list[str]:
    if not shutil.which("openssl"):
        return ["openssl is required to verify the checksum signature"]
    try:
        public_key = base64.b64decode(checksum_public_key, validate=True)
    except ValueError as error:
        return [f"invalid base64 checksum public key: {error}"]
    if len(public_key) != 32:
        return ["checksum public key must decode to a 32-byte Ed25519 public key"]

    with tempfile.TemporaryDirectory(prefix="wavecrate-published-signature-") as temp:
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
        return ["checksum signature verification failed: " + completed.stderr.strip()]
    return []


def validate_zip_manifest(
    zip_path: Path,
    zip_asset: ExpectedZip,
    archive_layout: ArchiveLayout,
    args: argparse.Namespace,
) -> list[str]:
    errors: list[str] = []
    try:
        with zipfile.ZipFile(zip_path) as archive:
            archive_entries = normalized_archive_entries(archive)
            archive_files = sorted(archive_entries)
            manifest_names = [name for name in archive_files if name.endswith("update-manifest.json")]
            if len(manifest_names) != 1:
                return [f"{zip_asset.name} must contain exactly one update-manifest.json"]
            manifest_archive_names = archive_entries[manifest_names[0]]
            if len(manifest_archive_names) != 1:
                return [f"{zip_asset.name} must contain exactly one update-manifest.json"]
            errors.extend(validate_archive_layout(zip_asset, archive_layout, archive_files, manifest_names[0]))
            manifest = json.loads(archive.read(manifest_archive_names[0]).decode("utf-8"))
    except zipfile.BadZipFile:
        return [f"{zip_asset.name} is not a valid zip file"]
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        return [f"{zip_asset.name} contains an invalid update-manifest.json: {error}"]

    expected = {
        "app": "wavecrate",
        "channel": args.channel,
        "target": zip_asset.target,
        "platform": zip_asset.platform,
        "arch": zip_asset.arch,
        "version": args.version,
        "target_version": args.target_version,
        "commit": args.commit,
        "build_date": args.build_date,
    }
    for key, expected_value in expected.items():
        if manifest.get(key) != expected_value:
            errors.append(
                f"{zip_asset.name} manifest {key} is {manifest.get(key)!r}, expected {expected_value!r}"
            )
    files = manifest.get("files")
    if not isinstance(files, list) or "update-manifest.json" not in files:
        errors.append(f"{zip_asset.name} manifest files must include update-manifest.json")
    else:
        errors.extend(validate_manifest_file_list(zip_asset, archive_layout, archive_files, files))
    return errors


def normalized_archive_entries(archive: zipfile.ZipFile) -> dict[str, list[str]]:
    entries: dict[str, list[str]] = {}
    for name in archive.namelist():
        normalized = normalize_archive_relative_path(name)
        if not normalized or name.endswith("/"):
            continue
        entries.setdefault(normalized, []).append(name)
    return entries


def validate_archive_layout(
    zip_asset: ExpectedZip,
    archive_layout: ArchiveLayout,
    archive_files: list[str],
    manifest_name: str,
) -> list[str]:
    errors: list[str] = []
    root = archive_layout.root_dir
    root_prefix = f"{root}/"
    expected_manifest = f"{root_prefix}update-manifest.json"
    if manifest_name != expected_manifest:
        errors.append(
            f"{zip_asset.name} manifest must be at {expected_manifest}, found {manifest_name}"
        )
    if not any(name.startswith(root_prefix) for name in archive_files):
        errors.append(f"{zip_asset.name} must contain expected archive root {root}/")

    allowed_prefixes = [root_prefix] + [f"{path.rstrip('/')}/" for path in archive_layout.optional_dirs]
    outside_root = [
        name
        for name in archive_files
        if not any(name == prefix.rstrip("/") or name.startswith(prefix) for prefix in allowed_prefixes)
    ]
    if outside_root:
        errors.append(
            f"{zip_asset.name} contains files outside expected archive root {root}/: "
            + ", ".join(outside_root[:5])
        )

    for expected_file in archive_layout.expected_files_for(zip_asset.platform):
        archive_path = f"{root_prefix}{expected_file}"
        if archive_path not in archive_files:
            errors.append(f"{zip_asset.name} is missing required archive file {archive_path}")
    return errors


def validate_manifest_file_list(
    zip_asset: ExpectedZip,
    archive_layout: ArchiveLayout,
    archive_files: list[str],
    manifest_files: list[Any],
) -> list[str]:
    if not all(isinstance(path, str) and path for path in manifest_files):
        return [f"{zip_asset.name} manifest files must be a string array"]
    root_prefix = f"{archive_layout.root_dir}/"
    actual = {
        name.removeprefix(root_prefix)
        for name in archive_files
        if name.startswith(root_prefix)
    }
    expected = {normalize_archive_relative_path(path) for path in manifest_files}
    missing = sorted(actual - expected)
    extra = sorted(expected - actual)
    errors: list[str] = []
    if missing:
        errors.append(
            f"{zip_asset.name} manifest files are missing archive entries: " + ", ".join(missing[:5])
        )
    if extra:
        errors.append(
            f"{zip_asset.name} manifest files lists entries absent from archive: " + ", ".join(extra[:5])
        )
    return errors


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


def fetch_json(url: str | None) -> dict[str, Any]:
    if not url:
        raise SystemExit("Missing URL for JSON fetch")
    try:
        with urllib.request.urlopen(request_for_url(url), timeout=60) as response:
            return json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, json.JSONDecodeError, UnicodeDecodeError) as error:
        raise SystemExit(f"Failed to fetch JSON from {url}: {error}") from error


def download_url(url: str, path: Path) -> None:
    try:
        with urllib.request.urlopen(request_for_url(url), timeout=120) as response:
            with path.open("wb") as output:
                shutil.copyfileobj(response, output)
    except urllib.error.URLError as error:
        raise SystemExit(f"Failed to download {url}: {error}") from error


def request_for_url(url: str) -> urllib.request.Request:
    return urllib.request.Request(url, headers={"User-Agent": "wavecrate-release-verifier"})


def fetch_portalsurfer_download_token(args: argparse.Namespace) -> str:
    response = fetch_json(portalsurfer_gate_url(args))
    if response.get("build_id") != args.portal_build_id:
        raise SystemExit(
            f"PortalSurfer gate build id mismatch: {response.get('build_id')} != {args.portal_build_id}"
        )
    token = response.get("download_token")
    if not isinstance(token, str) or not token:
        raise SystemExit("PortalSurfer gate did not return a download token")
    return token


def portalsurfer_gate_url(args: argparse.Namespace) -> str:
    releases_base = (args.portal_catalog_url or "https://portalsurfer.org/wavecrate/api/v1/releases").rstrip("/")
    build_id = urllib.parse.quote(str(args.portal_build_id), safe="")
    return f"{releases_base}/{build_id}/gate?donation_amount=0.00"


def portalsurfer_download_url(url: str, download_token: str) -> str:
    parsed = urllib.parse.urlparse(url)
    query = urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
    query = [(key, value) for key, value in query if key != "download_token"]
    query.append(("download_token", download_token))
    return urllib.parse.urlunparse(parsed._replace(query=urllib.parse.urlencode(query)))


def resolve_portalsurfer_url(args: argparse.Namespace, url: str) -> str:
    if urllib.parse.urlparse(url).scheme:
        return url
    base = args.portal_catalog_url or "https://portalsurfer.org/wavecrate/api/v1/releases"
    return urllib.parse.urljoin(base, url)


def print_errors(errors: list[str]) -> None:
    print("Published release verification failed:", file=sys.stderr)
    for error in errors:
        print(f" - {error}", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
