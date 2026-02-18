#!/usr/bin/env bash

# Runs Sempal in an isolated sandbox config directory so local runs (including
# agent runs) do not touch real user data.
#
# This works by setting `SEMPAL_CONFIG_HOME` to a fresh temporary directory.
# Sempal then creates and uses `<SEMPAL_CONFIG_HOME>/.sempal/` for config/logs.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SANDBOX_BASE=""
CLEAN=0

usage() {
  cat <<'EOF'
Usage: scripts/run_sandbox.sh [--dir <sandbox_base>] [--clean] [--] [app args...]

Runs `cargo run --release` with:
- `SEMPAL_CONFIG_HOME` set to an isolated sandbox base directory

Derived paths:
- app root:  <SEMPAL_CONFIG_HOME>/.sempal
- config:    <app root>/config.toml
- logs:      <app root>/logs

Options:
  --dir <path>  Use a fixed sandbox base dir (default: mktemp).
  --clean       Delete the sandbox base dir on exit.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --dir)
      SANDBOX_BASE="${2:-}"; shift 2 ;;
    --clean)
      CLEAN=1; shift ;;
    --)
      shift; break ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      break ;;
  esac
done

if [[ -z "$SANDBOX_BASE" ]]; then
  SANDBOX_BASE="$(mktemp -d -t sempal-sandbox-XXXXXXXX)"
else
  mkdir -p "$SANDBOX_BASE"
fi

if (( CLEAN == 1 )); then
  trap 'rm -rf "$SANDBOX_BASE"' EXIT
fi

export SEMPAL_CONFIG_HOME="$SANDBOX_BASE"

APP_ROOT="${SEMPAL_CONFIG_HOME}/.sempal"
CONFIG_PATH="${APP_ROOT}/config.toml"
LOGS_DIR="${APP_ROOT}/logs"

echo "[run_sandbox] repo_root=$ROOT_DIR"
echo "[run_sandbox] SEMPAL_CONFIG_HOME=$SEMPAL_CONFIG_HOME"
echo "[run_sandbox] app_root=$APP_ROOT"
echo "[run_sandbox] config=$CONFIG_PATH"
echo "[run_sandbox] logs=$LOGS_DIR"
echo "[run_sandbox] NOTE: this run uses an isolated config/log directory."

exec cargo run --release -- "$@"

