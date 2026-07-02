#!/usr/bin/env python3
"""Prepare a Wavecrate release train branch and package versions."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
PACKAGE_HEADER_RE = re.compile(r"^\[package\]\s*$")
TABLE_HEADER_RE = re.compile(r"^\[.*\]\s*$")
VERSION_LINE_RE = re.compile(r'^version\s*=\s*"([^"]+)"\s*$')
RELEASE_PACKAGE_RE = re.compile(r"^wavecrate($|-)")


@dataclass(frozen=True)
class ReleasePackage:
    name: str
    version: str
    manifest_path: Path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Prepare release/X.Y by aligning Wavecrate package versions and Cargo.lock."
    )
    parser.add_argument(
        "--version",
        required=True,
        help="Target release version, for example 19.1.0.",
    )
    parser.add_argument(
        "--source-ref",
        default="HEAD",
        help="Commit, branch, or tag used as the release branch base. Defaults to HEAD.",
    )
    parser.add_argument(
        "--branch",
        help="Release branch to create or update. Defaults to release/X.Y derived from --version.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Validate the current checkout without switching branches, writing files, committing, or pushing.",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="Push the prepared release branch to origin. This is never implied.",
    )
    parser.add_argument(
        "--skip-release-tests",
        action="store_true",
        help="Skip focused release tests. Intended only for script fixture tests.",
    )
    args = parser.parse_args()

    repo = repo_root()
    os.chdir(repo)
    version = args.version
    validate_version(version)
    branch = args.branch or release_branch_for_version(version)
    validate_branch(branch, version)
    ensure_clean_worktree()
    source_commit = git_stdout("rev-parse", "--verify", f"{args.source_ref}^{{commit}}")

    if args.dry_run:
        print(f"[release-train] dry run for {version} from {source_commit} into {branch}")
    else:
        git("switch", "-C", branch, source_commit)
        print(f"[release-train] preparing {branch} from {source_commit}")

    packages = release_packages(repo)
    if not packages:
        raise SystemExit("No Wavecrate release packages were found in the workspace.")
    reject_prerelease_versions(packages)
    changed_manifests = update_release_manifests(packages, version, dry_run=args.dry_run)

    if not args.dry_run:
        refresh_lockfile()
        validate_release_package_versions(version, release_packages(repo))

    validate_locked_metadata()
    if args.dry_run:
        reject_prerelease_lockfile_versions()
    else:
        validate_lockfile_versions(version)
    if not args.skip_release_tests:
        run_focused_release_tests()

    if not args.dry_run:
        if changed_manifests or git_has_changes():
            git(
                "add",
                *(str(package.manifest_path.relative_to(repo)) for package in packages),
                "Cargo.lock",
            )
            git("commit", "-m", f"Prepare Wavecrate {version} release train")
            print(f"[release-train] committed Wavecrate {version} release train prep")
        else:
            print("[release-train] no manifest or lockfile changes were needed")

    if args.push:
        if args.dry_run:
            raise SystemExit("--push cannot be combined with --dry-run")
        git("push", "-u", "origin", branch)
        print(f"[release-train] pushed {branch} to origin")
    else:
        print("[release-train] push skipped; pass --push to publish the release branch")

    print(f"[release-train] {branch} is prepared for {version}")
    return 0


def repo_root() -> Path:
    return Path(git_stdout("rev-parse", "--show-toplevel")).resolve()


def git(*args: str) -> None:
    run(["git", *args])


def git_stdout(*args: str) -> str:
    return run(["git", *args], capture=True).stdout.strip()


def cargo(*args: str) -> None:
    run(["cargo", *args])


def cargo_json(*args: str) -> dict:
    stdout = run(["cargo", *args], capture=True).stdout
    return json.loads(stdout)


def run(argv: list[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        argv,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if capture:
            sys.stderr.write(result.stdout)
            sys.stderr.write(result.stderr)
        raise SystemExit(f"Command failed ({result.returncode}): {' '.join(argv)}")
    return result


def validate_version(version: str) -> None:
    if not VERSION_RE.fullmatch(version):
        raise SystemExit(
            "version must be MAJOR.MINOR.PATCH without alpha, beta, rc, or build metadata"
        )


def release_branch_for_version(version: str) -> str:
    major, minor, _patch = version.split(".")
    return f"release/{major}.{minor}"


def validate_branch(branch: str, version: str) -> None:
    expected = release_branch_for_version(version)
    if branch != expected:
        raise SystemExit(f"branch must be {expected} for version {version}")


def ensure_clean_worktree() -> None:
    status = git_stdout("status", "--short")
    if status:
        raise SystemExit(
            "release train prep requires a clean working tree before it can switch branches or update manifests"
        )


def release_packages(repo: Path) -> list[ReleasePackage]:
    metadata = cargo_json("metadata", "--format-version", "1", "--no-deps")
    workspace_members = set(metadata["workspace_members"])
    packages: list[ReleasePackage] = []
    for package in metadata["packages"]:
        manifest_path = Path(package["manifest_path"]).resolve()
        if package["id"] not in workspace_members:
            continue
        if "vendor" in manifest_path.relative_to(repo).parts:
            continue
        if not RELEASE_PACKAGE_RE.match(package["name"]):
            continue
        packages.append(
            ReleasePackage(
                name=package["name"],
                version=package["version"],
                manifest_path=manifest_path,
            )
        )
    return sorted(packages, key=lambda package: str(package.manifest_path))


def reject_prerelease_versions(packages: list[ReleasePackage]) -> None:
    stale = [
        f"{package.name}={package.version}"
        for package in packages
        if not VERSION_RE.fullmatch(package.version)
    ]
    if stale:
        raise SystemExit(
            "release package versions must be stable MAJOR.MINOR.PATCH before train prep; stale prerelease versions: "
            + ", ".join(stale)
        )


def update_release_manifests(
    packages: list[ReleasePackage], version: str, *, dry_run: bool
) -> list[Path]:
    changed: list[Path] = []
    for package in packages:
        updated = update_manifest_package_version(package.manifest_path, version, dry_run=dry_run)
        if updated:
            changed.append(package.manifest_path)
            verb = "would update" if dry_run else "updated"
            print(
                f"[release-train] {verb} {package.manifest_path}: "
                f"{package.version} -> {version}"
            )
    if not changed:
        print("[release-train] release package manifests already match the requested version")
    return changed


def update_manifest_package_version(manifest_path: Path, version: str, *, dry_run: bool) -> bool:
    lines = manifest_path.read_text(encoding="utf-8").splitlines(keepends=True)
    in_package = False
    for index, line in enumerate(lines):
        stripped = line.strip()
        if PACKAGE_HEADER_RE.fullmatch(stripped):
            in_package = True
            continue
        if in_package and TABLE_HEADER_RE.fullmatch(stripped):
            break
        if in_package and (match := VERSION_LINE_RE.fullmatch(stripped)):
            old_version = match.group(1)
            if old_version == version:
                return False
            line_ending = "\n" if line.endswith("\n") else ""
            lines[index] = f'version = "{version}"{line_ending}'
            if not dry_run:
                manifest_path.write_text("".join(lines), encoding="utf-8")
            return True
    raise SystemExit(f"Could not find [package] version in {manifest_path}")


def refresh_lockfile() -> None:
    cargo("generate-lockfile")
    run(["cargo", "metadata", "--format-version", "1", "--no-deps"], capture=True)


def validate_locked_metadata() -> None:
    run(["cargo", "metadata", "--locked", "--format-version", "1", "--no-deps"], capture=True)


def validate_release_package_versions(version: str, packages: list[ReleasePackage]) -> None:
    mismatches = [
        f"{package.name}={package.version}"
        for package in packages
        if package.version != version
    ]
    if mismatches:
        raise SystemExit(
            f"Wavecrate release package versions must all be {version}; mismatches: "
            + ", ".join(mismatches)
        )


def validate_lockfile_versions(version: str) -> None:
    mismatches: list[str] = []
    for package in lockfile_packages():
        name = package.get("name")
        package_version = package.get("version")
        if isinstance(name, str) and RELEASE_PACKAGE_RE.match(name) and package_version != version:
            mismatches.append(f"{name}={package_version}")
    if mismatches:
        raise SystemExit(
            f"Cargo.lock release package versions must all be {version}; mismatches: "
            + ", ".join(mismatches)
        )


def reject_prerelease_lockfile_versions() -> None:
    stale: list[str] = []
    for package in lockfile_packages():
        name = package.get("name")
        package_version = package.get("version")
        if (
            isinstance(name, str)
            and RELEASE_PACKAGE_RE.match(name)
            and isinstance(package_version, str)
            and not VERSION_RE.fullmatch(package_version)
        ):
            stale.append(f"{name}={package_version}")
    if stale:
        raise SystemExit(
            "Cargo.lock release package versions must be stable before train prep; stale prerelease versions: "
            + ", ".join(stale)
        )


def lockfile_packages() -> list[dict[str, str]]:
    packages: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    for raw_line in Path("Cargo.lock").read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if line == "[[package]]":
            if current is not None:
                packages.append(current)
            current = {}
            continue
        if current is None or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip()
        if key in {"name", "version"} and value.startswith('"') and value.endswith('"'):
            current[key] = value[1:-1]
    if current is not None:
        packages.append(current)
    return packages


def run_focused_release_tests() -> None:
    cargo("test", "--test", "release_contract", "--test", "manual_release_matching")


def git_has_changes() -> bool:
    return bool(git_stdout("status", "--short"))


if __name__ == "__main__":
    raise SystemExit(main())
