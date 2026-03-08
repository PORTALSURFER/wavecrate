#!/usr/bin/env bash

# Run the fastest local compile/smoke gate.
#
# This checks library, test, and binary targets without running the suite.
# Use this during the tight edit loop, then escalate to `ci_quick.sh` for
# filtered tests and `ci_local.sh` for full CI parity.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'EOF'
Usage: scripts/devcheck.sh

Run the fastest local compile/smoke gate.
For fast test coverage, use `scripts/ci_quick.sh`.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[devcheck] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[devcheck] cargo check --tests --bins"
cargo check --tests --bins

echo "[devcheck] OK"
