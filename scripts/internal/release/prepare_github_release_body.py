#!/usr/bin/env python3
"""Prepare a GitHub release body within GitHub's release body limit."""

from __future__ import annotations

import argparse
from pathlib import Path


DEFAULT_MAX_CHARS = 120_000
NOTICE_TEMPLATE = """

## Full Release Log

This generated release log exceeded GitHub's release-body limit and was
shortened for the GitHub release page. The complete log is attached as
`release-log.md` and is published in the PortalSurfer release changelog.
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Write a GitHub-safe release body from a canonical release log."
    )
    parser.add_argument("--input", required=True, help="Canonical release-log.md path.")
    parser.add_argument("--output", required=True, help="GitHub body output path.")
    parser.add_argument(
        "--max-chars",
        type=int,
        default=DEFAULT_MAX_CHARS,
        help=f"Maximum output characters. Defaults to {DEFAULT_MAX_CHARS}.",
    )
    return parser.parse_args()


def shorten_release_log(body: str, max_chars: int) -> str:
    if max_chars <= 0:
        raise ValueError("--max-chars must be positive")
    if len(body) <= max_chars:
        return body

    notice = NOTICE_TEMPLATE.strip("\n")
    suffix = "\n\n" + notice + "\n"
    if len(suffix) >= max_chars:
        raise ValueError("--max-chars is too small for the truncation notice")

    budget = max_chars - len(suffix)
    prefix = body[:budget].rstrip()
    return prefix + suffix


def main() -> int:
    args = parse_args()
    input_path = Path(args.input)
    output_path = Path(args.output)
    body = input_path.read_text(encoding="utf-8")
    output = shorten_release_log(body, args.max_chars)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(output, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
