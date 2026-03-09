#!/usr/bin/env bash

# Run the fast workspace-wide test loop.
#
# This executes the quick nextest profile across all workspace members and
# target kinds. Use it for package-shape and tooling edits where the app-only
# lane is insufficient.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/use_cargo_cache.sh
source "$ROOT_DIR/scripts/use_cargo_cache.sh"
sempal_enable_cargo_cache

usage() {
  cat <<'EOF'
Usage: scripts/ci_quick_workspace.sh

Run the fast nextest loop for all workspace packages.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[ci_quick_workspace] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

echo "[ci_quick_workspace] cargo nextest run --workspace --profile quick --all-targets"
cargo nextest run --workspace --profile quick --all-targets

echo "[ci_quick_workspace] OK"
