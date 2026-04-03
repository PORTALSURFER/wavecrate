#!/usr/bin/env bash

# Agent-safe local validation loop.
#
# This lane avoids cargo-nextest and the broader GUI contract wrappers so it
# can run in constrained environments while still providing a real compile +
# library-test cycle.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'USAGE'
Usage: scripts/ci_agent.sh

Run the agent-safe local validation loop without cargo-nextest.
For the broader integrated lane, use `scripts/ci_quick.sh`.
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
      echo "[ci_agent] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[ci_agent] ./scripts/check_next_branch.sh"
./scripts/check_next_branch.sh

echo "[ci_agent] ./scripts/devcheck.sh"
./scripts/devcheck.sh

echo "[ci_agent] cargo test -p sempal --lib"
cargo test -p sempal --lib

echo "[ci_agent] OK"
