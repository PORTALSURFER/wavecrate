#!/usr/bin/env bash

# Fast local development checks.
#
# This script keeps the everyday edit/test loop lean by running the filtered
# nextest quick profile over library and integration tests while skipping
# support-tool binaries and the slower CI-parity steps. Use scripts/ci_local.sh
# for the full gate.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'USAGE'
Usage: scripts/ci_quick.sh

Run the fast local development test loop.
For the compile-only smoke gate, use `scripts/devcheck.sh`.
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

echo "[ci_quick] branch policy"
./scripts/check_next_branch.sh

echo "[ci_quick] cargo nextest run -p sempal --profile quick --lib --tests"
cargo nextest run -p sempal --profile quick --lib --tests

echo "[ci_quick] OK"
