#!/usr/bin/env python3
"""Verify a PortalSurfer release-upload catalog round trip."""

from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Verify that a PortalSurfer catalog lists an uploaded release.",
    )
    parser.add_argument("--catalog-file", required=True, type=Path)
    parser.add_argument("--build-id", required=True)
    parser.add_argument("--build-number", required=True, type=int)
    parser.add_argument("--release-version", required=True)
    parser.add_argument("--released-at", required=True)
    parser.add_argument("--expected-file", action="append", default=[])
    return parser.parse_args()


def parse_released_at(value: Any, label: str) -> dt.datetime:
    if not isinstance(value, str) or not value:
        raise SystemExit(f"{label} is missing or not a string")
    normalized = value
    if normalized.endswith("Z"):
        normalized = f"{normalized[:-1]}+00:00"
    try:
        parsed = dt.datetime.fromisoformat(normalized)
    except ValueError as exc:
        raise SystemExit(f"{label} is not a valid ISO-8601 timestamp: {value}") from exc
    if parsed.tzinfo is None:
        raise SystemExit(f"{label} must include a timezone: {value}")
    return parsed.astimezone(dt.timezone.utc)


def main() -> int:
    args = parse_args()
    catalog = json.loads(args.catalog_file.read_text(encoding="utf-8"))
    releases = catalog.get("releases") or []
    release = next((item for item in releases if item.get("build_id") == args.build_id), None)
    if release is None:
        raise SystemExit(f"Release catalog does not list {args.build_id}")
    if release.get("build_number") != args.build_number:
        raise SystemExit(
            f"Release catalog build number mismatch: "
            f"{release.get('build_number')} != {args.build_number}"
        )
    if release.get("version") != args.release_version:
        raise SystemExit(
            f"Release catalog version mismatch: "
            f"{release.get('version')} != {args.release_version}"
        )
    actual_released_at = parse_released_at(release.get("released_at"), "Release catalog timestamp")
    expected_released_at = parse_released_at(args.released_at, "Expected release timestamp")
    if actual_released_at != expected_released_at:
        raise SystemExit(
            f"Release catalog timestamp mismatch: "
            f"{release.get('released_at')} != {args.released_at}"
        )
    expected_files = set(args.expected_file)
    catalog_files = {item.get("name") for item in release.get("files") or []}
    missing = sorted(expected_files - catalog_files)
    if missing:
        raise SystemExit(f"Release catalog is missing files: {', '.join(missing)}")
    changelog = release.get("changelog") or {}
    if changelog.get("format") != "markdown" or not changelog.get("url"):
        raise SystemExit("Release catalog is missing the markdown changelog link")
    print(f"Uploaded {len(expected_files)} Wavecrate release file(s) to {args.build_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
