#!/usr/bin/env python3
"""Emit the pinned Rust toolchain channel for GitHub Actions."""

from __future__ import annotations

import re
from pathlib import Path


def main() -> int:
    text = Path("rust-toolchain.toml").read_text(encoding="utf-8")
    channel = "stable"
    in_toolchain = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line == "[toolchain]":
            in_toolchain = True
            continue
        if in_toolchain and line.startswith("["):
            break
        if in_toolchain and (match := re.fullmatch(r'channel\s*=\s*"([^"]+)"', line)):
            channel = match.group(1)
            break
    print(f"channel={channel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
