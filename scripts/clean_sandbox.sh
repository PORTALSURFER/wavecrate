#!/usr/bin/env bash

# Deletes the repo-local sandbox used by `scripts/run_sandbox.*`.
#
# This removes:
# - <repo>/.sandbox/sempal
#
# Use this when sandbox state gets confusing or you want a fresh sandbox run.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SANDBOX_DIR="$ROOT_DIR/.sandbox/sempal"

if [[ ! -e "$SANDBOX_DIR" ]]; then
  echo "[clean_sandbox] nothing to remove: $SANDBOX_DIR"
  exit 0
fi

echo "[clean_sandbox] removing: $SANDBOX_DIR"
rm -rf "$SANDBOX_DIR"
echo "[clean_sandbox] OK"

