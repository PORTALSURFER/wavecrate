#!/usr/bin/env bash

# Fast local development checks.
#
# This script keeps the everyday edit/test loop lean by running the normal
# unit, integration, and binary tests through cargo-nextest while skipping the
# slower CI-parity steps. Use scripts/ci_local.sh for the full gate.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/ci_quick.sh

Run the fast local development test loop.

For full CI parity, use `scripts/ci_local.sh`.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[ci_quick] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[ci_quick] cargo nextest run --lib --bins --tests --no-fail-fast"
cargo nextest run --lib --bins --tests --no-fail-fast

echo "[ci_quick] OK"
