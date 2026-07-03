#!/usr/bin/env python3
"""Assemble the PortalSurfer full changelog from per-release logs."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from urllib.error import HTTPError
from pathlib import Path
from typing import Any


USER_AGENT = "wavecrate-release-changelog-assembler"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build a site-wide Wavecrate changelog by concatenating the "
            "markdown logs bound to each PortalSurfer release."
        )
    )
    catalog = parser.add_mutually_exclusive_group(required=True)
    catalog.add_argument("--catalog-file", help="Path to a release catalog JSON file.")
    catalog.add_argument("--catalog-url", help="Public release catalog URL.")
    parser.add_argument("--current-build-id", required=True)
    parser.add_argument("--current-build-number", type=int, required=True)
    parser.add_argument("--current-version", required=True)
    parser.add_argument("--current-released-at", required=True)
    parser.add_argument("--current-log", required=True, help="Markdown log for the current release.")
    parser.add_argument(
        "--existing-changelog-url",
        help="Existing site-wide changelog endpoint used to preserve prior release logs.",
    )
    parser.add_argument("--output", required=True)
    parser.add_argument(
        "--generated-at",
        help="ISO-8601 timestamp to write in the changelog metadata. Defaults to now.",
    )
    parser.add_argument(
        "--request-delay-seconds",
        type=float,
        default=0.25,
        help="Delay between public changelog fetches. Defaults to 0.25 seconds.",
    )
    return parser.parse_args()


def read_url(url: str, *, request_delay_seconds: float = 0.0) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(5):
        if request_delay_seconds > 0:
            time.sleep(request_delay_seconds)
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.read().decode("utf-8")
        except HTTPError as error:
            if error.code != 429 or attempt == 4:
                raise
            retry_after = error.headers.get("Retry-After")
            if retry_after is not None and retry_after.isdigit():
                delay = float(retry_after)
            else:
                delay = 2.0 * (attempt + 1)
            time.sleep(delay)
    raise RuntimeError(f"Unable to read {url}")


def load_json(source: str, *, request_delay_seconds: float = 0.0) -> dict[str, Any]:
    if source.startswith(("http://", "https://")):
        return json.loads(read_url(source, request_delay_seconds=request_delay_seconds))
    return json.loads(Path(source).read_text(encoding="utf-8"))


def changelog_url(catalog_url: str | None, release: dict[str, Any]) -> str | None:
    changelog = release.get("changelog") or {}
    url = changelog.get("url")
    if not isinstance(url, str) or not url:
        return None
    if url.startswith(("http://", "https://")):
        return url
    if catalog_url is None:
        return None
    return urllib.parse.urljoin(catalog_url, url)


def release_version(build_id: str) -> str:
    prefix = "wavecrate-"
    return build_id[len(prefix) :] if build_id.startswith(prefix) else build_id


def release_header_versions(release: dict[str, Any]) -> list[str]:
    build_id = release.get("build_id")
    if not isinstance(build_id, str) or not build_id:
        return []
    versions = {release_version(build_id)}
    version = release.get("version")
    if isinstance(version, str) and version.strip():
        versions.add(version.strip())
    changelog = release.get("changelog") or {}
    title = changelog.get("title")
    if isinstance(title, str) and title.startswith("Wavecrate "):
        versions.add(title.removeprefix("Wavecrate ").strip())
    build_number = release.get("build_number")
    suffix = build_id.rsplit("-", 1)[-1]
    if (
        isinstance(build_number, int)
        and re.fullmatch(r"[0-9a-fA-F]{7,40}", suffix)
        and not release_version(build_id).startswith(f"nightly-b{build_number}-")
    ):
        versions.add(f"nightly-b{build_number}-{suffix}")
    return sorted(version for version in versions if version)


def assert_bound_header(build_id: str, body: str, versions: list[str]) -> None:
    patterns = []
    for version in versions:
        escaped = re.escape(version)
        patterns.extend(
            [
                rf"(?m)^#{{1,6}}\s+Wavecrate\s+{escaped}\s*$",
                rf"(?m)^#{{1,6}}\s+\[{escaped}\](?:\s+-\s+.*)?$",
            ]
        )
    if any(re.search(pattern, body) for pattern in patterns):
        return
    raise SystemExit(
        f"Release log for {build_id} does not contain a markdown header "
        f"bound to one of: {', '.join(versions)}."
    )


def release_sort_key(release: dict[str, Any]) -> tuple[str, int]:
    released_at = release.get("released_at")
    build_number = release.get("build_number")
    return (
        released_at if isinstance(released_at, str) else "",
        build_number if isinstance(build_number, int) else -1,
    )


def current_release_from_args(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "build_id": args.current_build_id,
        "build_number": args.current_build_number,
        "version": args.current_version,
        "released_at": args.current_released_at,
        "changelog": {
            "title": f"Wavecrate {release_version(args.current_build_id)}",
            "format": "markdown",
        },
    }


def load_release_body(
    release: dict[str, Any],
    *,
    current_build_id: str,
    current_body: str,
    catalog_url: str | None,
    request_delay_seconds: float,
) -> str | None:
    build_id = release.get("build_id")
    if build_id == current_build_id:
        return current_body
    changelog = release.get("changelog") or {}
    body = changelog.get("body")
    if isinstance(body, str) and body.strip():
        return body
    url = changelog_url(catalog_url, release)
    if url is None:
        return None
    response = load_json(url, request_delay_seconds=request_delay_seconds)
    body = (response.get("changelog") or {}).get("body")
    return body if isinstance(body, str) and body.strip() else None


def existing_changelog_body(url: str, *, request_delay_seconds: float) -> str:
    response = load_json(url, request_delay_seconds=request_delay_seconds)
    body = (response.get("changelog") or {}).get("body")
    return body if isinstance(body, str) else ""


def release_heading_pattern() -> re.Pattern[str]:
    return re.compile(
        r"(?m)^(?=(?:#\s+Wavecrate\s+(?!Changelog\b)|##\s+\[nightly-))"
    )


def strip_global_changelog_header(body: str) -> str:
    body = body.strip()
    if not body.startswith("# Wavecrate Changelog"):
        return body
    match = release_heading_pattern().search(body)
    if match is None:
        return ""
    return body[match.start() :].strip()


def remove_release_sections(body: str, versions: list[str]) -> str:
    body = strip_global_changelog_header(body)
    if not body:
        return ""
    sections = [
        section.strip()
        for section in release_heading_pattern().split(body)
        if section.strip()
    ]
    kept = []
    for section in sections:
        first_line = section.splitlines()[0] if section.splitlines() else ""
        if any(version in first_line for version in versions):
            continue
        kept.append(section)
    return "\n\n".join(kept).strip()


def main() -> int:
    args = parse_args()
    catalog_url = args.catalog_url
    catalog = load_json(
        args.catalog_url or args.catalog_file,
        request_delay_seconds=args.request_delay_seconds,
    )
    releases = catalog.get("releases")
    if not isinstance(releases, list):
        raise SystemExit("Release catalog does not contain a releases array.")

    current_release = current_release_from_args(args)
    releases = [
        release
        for release in releases
        if release.get("build_id") != args.current_build_id
    ]
    releases.append(current_release)
    sorted_releases = sorted(releases, key=release_sort_key, reverse=True)
    current_release = next(
        (release for release in sorted_releases if release.get("build_id") == args.current_build_id),
        None,
    )
    if current_release is None:
        raise SystemExit(f"Release catalog does not list {args.current_build_id}.")
    current_body = Path(args.current_log).read_text(encoding="utf-8").strip()
    if not current_body:
        raise SystemExit("Current release log is empty.")
    assert_bound_header(
        args.current_build_id,
        current_body,
        release_header_versions(current_release),
    )

    generated_at = args.generated_at
    if generated_at is None:
        generated_at = (
            dt.datetime.now(dt.timezone.utc)
            .replace(microsecond=0)
            .isoformat()
            .replace("+00:00", "Z")
        )

    output: list[str] = [
        "# Wavecrate Changelog",
        "",
        f"- Latest release: {args.current_build_id}",
        f"- Latest build: {current_release.get('build_number')}",
        f"- Latest version: {current_release.get('version')}",
        f"- Generated: {generated_at}",
        "",
    ]

    current_versions = release_header_versions(current_release)
    if args.existing_changelog_url:
        existing_body = existing_changelog_body(
            args.existing_changelog_url,
            request_delay_seconds=args.request_delay_seconds,
        )
        previous_body = remove_release_sections(existing_body, current_versions)
        if previous_body and not previous_body.startswith("# Wavecrate "):
            raise SystemExit(
                "Existing full changelog is not release-bound. Repair it so "
                "each historical log starts with '# Wavecrate ...' "
                "before the nightly workflow maintains it."
            )
        output.append(current_body)
        if previous_body:
            output.extend(["", previous_body])
        Path(args.output).write_text("\n".join(output).rstrip() + "\n", encoding="utf-8")
        print(f"Assembled current Wavecrate release log into {args.output}")
        return 0

    written = 0
    for release in sorted_releases:
        build_id = release.get("build_id")
        if not isinstance(build_id, str) or not build_id:
            continue
        body = load_release_body(
            release,
            current_build_id=args.current_build_id,
            current_body=current_body,
            catalog_url=catalog_url,
            request_delay_seconds=args.request_delay_seconds,
        )
        if body is None:
            output.extend(
                [
                    f"## Wavecrate {release_version(build_id)}",
                    "",
                    "Release log unavailable.",
                    "",
                ]
            )
            written += 1
            continue
        body = body.strip()
        assert_bound_header(build_id, body, release_header_versions(release))
        output.extend([body, ""])
        written += 1

    if written == 0:
        raise SystemExit("No release logs were written to the full changelog.")

    Path(args.output).write_text("\n".join(output).rstrip() + "\n", encoding="utf-8")
    print(f"Assembled {written} Wavecrate release log(s) into {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
