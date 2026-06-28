#!/usr/bin/env bash

set -euo pipefail

if ! command -v mdbook >/dev/null 2>&1; then
  echo "[mdbook] ERROR: mdbook is required. Install with: cargo install mdbook --locked" >&2
  exit 2
fi

mdbook build
echo "[mdbook] OK"
