#!/usr/bin/env bash

# Reports broken local file links in Markdown across the documentation surface.
#
# Scans:
# - README.md
# - docs/**/*.md
# - manual/**/*.md
#
# Ignores:
# - HTTP(S), mailto, tel
# - Absolute site links starting with `/`
# - Pure anchors starting with `#`

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

python3 - "$timestamp" <<'PY'
import re
import sys
from pathlib import Path

timestamp = sys.argv[1]
ROOT = Path.cwd()

def iter_md_files():
    yield ROOT / "README.md"
    for p in (ROOT / "docs").rglob("*.md"):
        yield p
    for p in (ROOT / "manual").rglob("*.md"):
        yield p

def is_ignored(dest: str) -> bool:
    dest = dest.strip()
    if not dest:
        return True
    lower = dest.lower()
    if lower.startswith(("http://", "https://", "mailto:", "tel:")):
        return True
    if dest.startswith("#"):
        return True
    if dest.startswith("/"):
        return True
    return False

def strip_anchor_and_query(dest: str) -> str:
    dest = dest.split("#", 1)[0]
    dest = dest.split("?", 1)[0]
    return dest.strip()

link_re = re.compile(r"!?\\[[^\\]]*\\]\\(([^)]+)\\)")

violations = []
checked_files = 0
checked_links = 0

for path in iter_md_files():
    if not path.is_file():
        continue
    checked_files += 1
    text = path.read_text(encoding="utf-8", errors="replace")
    for match in link_re.finditer(text):
        dest_raw = match.group(1).strip()
        if is_ignored(dest_raw):
            continue
        dest = strip_anchor_and_query(dest_raw)
        if not dest or is_ignored(dest):
            continue
        if "${" in dest or "{{" in dest:
            continue
        checked_links += 1
        resolved = (path.parent / dest).resolve()
        if not resolved.exists():
            violations.append((str(path.relative_to(ROOT)), dest_raw))

print("# Markdown link report")
print()
print(f"- Timestamp (UTC): `{timestamp}`")
print(f"- Files scanned: `{checked_files}`")
print(f"- Local file links checked: `{checked_links}`")
print()

print("## Broken local links")
print()
if not violations:
    print("None.")
    sys.exit(0)

print("| File | Link |")
print("| --- | --- |")
for file_path, dest in sorted(violations):
    print(f"| `{file_path}` | `{dest}` |")
PY

