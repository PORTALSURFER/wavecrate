#!/usr/bin/env bash

# Broader integrated local development checks.
#
# This Unix wrapper runs the quick nextest lane only. It intentionally skips
# the slower CI-parity steps and does not include the Windows-only semantic GUI
# contract lane that the PowerShell wrapper adds. Use scripts/ci_local.sh for
# the fuller Unix gate.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'USAGE'
Usage: scripts/ci_quick.sh

Run the broader integrated local development test loop.
This Unix wrapper runs the quick nextest lane only and skips the Windows-only
semantic GUI contract step that `scripts/ci_quick.ps1` includes.
For the constrained agent-safe lane, use `scripts/ci_agent.sh`.
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
