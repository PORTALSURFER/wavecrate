#!/usr/bin/env bash

# Run the broad workspace compile/smoke gate.
#
# This checks test and binary targets across every Cargo workspace member.
# Use it for package-split and support-tool changes that should not rely on the
# app-only default member lane.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'EOF'
Usage: scripts/devcheck_workspace.sh

Run the broad compile/smoke gate for all workspace packages.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[devcheck_workspace] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[devcheck_workspace] cargo check --workspace --tests --bins"
cargo check --workspace --tests --bins

echo "[devcheck_workspace] OK"
