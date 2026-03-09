#!/usr/bin/env bash

# Run the lightest app-only compile gate.
#
# This path checks the main library plus the `sempal` application binary without
# building support-tool bins or test targets. Use it before `devcheck.sh` when
# you are only touching normal app/runtime code.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'EOF'
Usage: scripts/devcheck_app.sh

Run the lightest app-only compile gate.
Use this before `scripts/devcheck.sh` when you are only changing the main app.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[devcheck_app] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[devcheck_app] cargo check -p sempal --lib --bin sempal"
cargo check -p sempal --lib --bin sempal

echo "[devcheck_app] OK"
