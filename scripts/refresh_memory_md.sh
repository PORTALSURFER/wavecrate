#!/usr/bin/env bash

# Update MEMORY.md with a fresh timestamp and updater identity.
#
# By default this sets "Last Updated" to current UTC time and "Updated By" to
# "Codex", while preserving the rest of the document.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MEMORY_FILE="MEMORY.md"
UPDATER="${1:-Codex}"
TIMESTAMP="${2:-$(date -u +'%Y-%m-%dT%H:%M:%SZ')}"

if [[ ! -f "$MEMORY_FILE" ]]; then
  echo "[memory_refresh] Missing required file: $MEMORY_FILE" >&2
  exit 1
fi

python3 - "$MEMORY_FILE" "$TIMESTAMP" "$UPDATER" <<'PY'
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
timestamp = sys.argv[2]
updated_by = sys.argv[3]

text = path.read_text(encoding="utf-8").splitlines()
found_timestamp = False
found_updated_by = False
updated = []

for line in text:
    if line.startswith("Last Updated:"):
        updated.append(f"Last Updated: {timestamp}")
        found_timestamp = True
    elif line.startswith("Updated By:"):
        updated.append(f"Updated By: {updated_by}")
        found_updated_by = True
    else:
        updated.append(line)

if not found_timestamp:
    updated.append(f"Last Updated: {timestamp}")
if not found_updated_by:
    updated.append(f"Updated By: {updated_by}")

path.write_text("\n".join(updated) + "\n", encoding="utf-8")
PY

echo "[memory_refresh] Updated $MEMORY_FILE (Last Updated: $TIMESTAMP, Updated By: $UPDATER)"
