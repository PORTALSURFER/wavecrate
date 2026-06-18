#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: ./run.sh [--] [app args...]

Builds and runs a release-profile internal Wavecrate dev build with logging
enabled. App args are forwarded after the built-in --log flag.
EOF
}

if (( $# > 0 )); then
  case "$1" in
    -h|--help|-Help|help)
      usage
      exit 0
      ;;
    --)
      shift
      ;;
  esac
fi

cd "$ROOT_DIR"

if [[ ! -f "$ROOT_DIR/vendor/radiant/Cargo.toml" ]]; then
  echo "[run] Radiant submodule is missing; initializing vendor/radiant..."
  git submodule update --init --recursive vendor/radiant
fi

export WAVECRATE_INTERNAL_BUILD=1
exec cargo run -r -- --log "$@"
