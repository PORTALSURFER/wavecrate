#!/usr/bin/env bash

# Deletes the repo-local sandbox used by `scripts/run_sandbox.*`.
#
# This removes:
# - <repo>/.sandbox/wavecrate
#
# Use this when sandbox state gets confusing or you want a fresh sandbox run.

set -euo pipefail

usage() {
  local entrypoint="${WAVECRATE_RUN_ENTRYPOINT:-scripts/run.sh}"
  cat <<EOF
Usage: ${entrypoint} clean

Deletes the repo-local sandbox used by ${entrypoint} sandbox.
EOF
}

if (( $# > 0 )); then
  case "$1" in
    -h|--help|-Help)
      usage
      exit 0
      ;;
    *)
      echo "[clean_sandbox] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

SANDBOX_DIR="$ROOT_DIR/.sandbox/wavecrate"

if [[ ! -e "$SANDBOX_DIR" ]]; then
  echo "[clean_sandbox] nothing to remove: $SANDBOX_DIR"
  exit 0
fi

echo "[clean_sandbox] removing: $SANDBOX_DIR"
rm -rf "$SANDBOX_DIR"
echo "[clean_sandbox] OK"
