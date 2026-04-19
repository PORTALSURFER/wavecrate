#!/usr/bin/env bash

# Broader integrated local development checks.
#
# This Unix wrapper runs the quick nextest lane only. It intentionally skips
# the slower CI-parity steps and does not include the Windows-only semantic GUI
# contract lane that the PowerShell wrapper adds. Use scripts/ci.sh local for
# the fuller Unix gate.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/internal/use_cargo_cache.sh
source "$ROOT_DIR/scripts/internal/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'USAGE'
Usage: scripts/ci.sh quick [--workspace]

Run the broader integrated local development test loop.
This Unix wrapper runs the quick nextest lane only and skips the Windows-only
semantic GUI contract step that `scripts/ci.ps1 quick` includes.
Use `--workspace` for the full workspace nextest lane.
For the constrained agent-safe lane, use `scripts/ci.sh agent`.
For the compile-only smoke gate, use `scripts/ci.sh smoke`.
For full CI parity, use `scripts/ci.sh local`.
USAGE
}

WORKSPACE=0

while (( $# > 0 )); do
  case "$1" in
    --workspace)
      WORKSPACE=1
      shift
      ;;
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
./scripts/internal/check/check_next_branch.sh

if (( WORKSPACE == 1 )); then
  echo "[ci_quick] cargo nextest run --workspace --profile quick --all-targets"
  cargo nextest run --workspace --profile quick --all-targets
else
  echo "[ci_quick] cargo nextest run -p sempal --profile quick --lib --tests"
  cargo nextest run -p sempal --profile quick --lib --tests
fi

echo "[ci_quick] OK"
