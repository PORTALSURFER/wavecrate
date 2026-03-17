#!/usr/bin/env bash

# Broader integrated local development checks.
#
# This script runs the broader integrated local lane by using the filtered
# nextest quick profile while still skipping the slower CI-parity steps. The
# Windows PowerShell wrapper also includes the semantic GUI contract suite. Use
# scripts/ci_local.sh for the full gate.

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
