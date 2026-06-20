#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<'EOF'
Usage: ./run.sh [--] [app args...]

Builds and runs a release-profile Wavecrate dev build with logging.
On macOS, an available dev-app helper stages and opens a
LaunchServices-visible target/dev-app/Wavecrate.app; otherwise this falls back
to cargo run. App args are forwarded after the built-in --log flag.
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

DEV_APP_BUNDLE_SCRIPT="$ROOT_DIR/scripts/internal/run/dev_app_bundle.sh"
if [[ "$(uname -s)" == "Darwin" && "${WAVECRATE_DIRECT_RUN:-}" != "1" && -x "$DEV_APP_BUNDLE_SCRIPT" ]]; then
  exec "$DEV_APP_BUNDLE_SCRIPT" "$@"
fi

exec cargo run -r -- --log "$@"
