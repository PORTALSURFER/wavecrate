#!/usr/bin/env bash

# Runs Wavecrate in an isolated sandbox config directory so local runs (including
# agent runs) do not touch real user data.
#
# This works by setting `WAVECRATE_CONFIG_HOME` to a sandbox base directory and
# `WAVECRATE_CONFIG_PROFILE=sandbox`, which routes config/logs/library state into
# `<WAVECRATE_CONFIG_HOME>/.wavecrate/profiles/sandbox/`.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

SANDBOX_BASE=""
CLEAN=0
TEMP=0
NAME=""
WRITE_DB=0
ALLOW_USER_LIBRARY_DB_WRITE=0

usage() {
  local entrypoint="${WAVECRATE_RUN_ENTRYPOINT:-scripts/run.sh}"
  cat <<EOF
Usage: ${entrypoint} sandbox [--dir <sandbox_base>] [--name <id>] [--temp] [--clean] [--write-db] [--allow-user-library-db-write] [--] [app args...]

Runs cargo run --release with:
- WAVECRATE_CONFIG_HOME set to an isolated sandbox base directory
- WAVECRATE_CONFIG_PROFILE=sandbox

Derived paths:
- app root:  <WAVECRATE_CONFIG_HOME>/.wavecrate/profiles/sandbox
- config:    <app root>/config.toml
- logs:      <app root>/logs

Options:
  --dir <path>  Use a fixed sandbox base dir (default: <repo>/.sandbox/wavecrate).
  --name <id>   Use a named sandbox dir under <repo>/.sandbox/wavecrate/<id> (persistent).
  --temp        Use a temporary sandbox base dir (mktemp) and delete it on exit.
  --clean       Delete the sandbox base dir on exit.
  --write-db    Allow source DB writes (opt out of read-only DB mode).
  --allow-user-library-db-write
                Allow DB writes under user-library-like source paths.
                Ignored unless --write-db is also provided.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --dir)
      SANDBOX_BASE="${2:-}"; shift 2 ;;
    --name)
      NAME="${2:-}"; shift 2 ;;
    --temp)
      TEMP=1; shift ;;
    --clean)
      CLEAN=1; shift ;;
    --write-db)
      WRITE_DB=1; shift ;;
    --allow-user-library-db-write)
      ALLOW_USER_LIBRARY_DB_WRITE=1; shift ;;
    --)
      shift; break ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      break ;;
  esac
done

if (( TEMP == 1 )) && [[ -n "$NAME" ]]; then
  echo "[run_sandbox][error] --temp and --name are mutually exclusive." >&2
  exit 2
fi
if [[ -n "$SANDBOX_BASE" ]] && [[ -n "$NAME" ]]; then
  echo "[run_sandbox][error] --dir and --name are mutually exclusive." >&2
  exit 2
fi

if [[ -z "$SANDBOX_BASE" ]]; then
  if (( TEMP == 1 )); then
    SANDBOX_BASE="$(mktemp -d -t wavecrate-sandbox-XXXXXXXX)"
    CLEAN=1
  elif [[ -n "$NAME" ]]; then
    SANDBOX_BASE="$ROOT_DIR/.sandbox/wavecrate/$NAME"
  else
    SANDBOX_BASE="$ROOT_DIR/.sandbox/wavecrate"
  fi
fi

mkdir -p "$SANDBOX_BASE"

if (( CLEAN == 1 )); then
  trap 'rm -rf "$SANDBOX_BASE"' EXIT
fi

export WAVECRATE_CONFIG_HOME="$SANDBOX_BASE"
export WAVECRATE_CONFIG_PROFILE="sandbox"
if (( WRITE_DB == 1 )); then
  unset WAVECRATE_SOURCE_DB_READ_ONLY
else
  export WAVECRATE_SOURCE_DB_READ_ONLY=1
fi

if (( ALLOW_USER_LIBRARY_DB_WRITE == 1 )); then
  export WAVECRATE_ALLOW_USER_LIBRARY_DB_WRITE=1
else
  unset WAVECRATE_ALLOW_USER_LIBRARY_DB_WRITE
fi

APP_ROOT="${WAVECRATE_CONFIG_HOME}/.wavecrate/profiles/sandbox"
CONFIG_PATH="${APP_ROOT}/config.toml"
LOGS_DIR="${APP_ROOT}/logs"

echo "[run_sandbox] repo_root=$ROOT_DIR"
echo "[run_sandbox] WAVECRATE_CONFIG_HOME=$WAVECRATE_CONFIG_HOME"
echo "[run_sandbox] WAVECRATE_CONFIG_PROFILE=$WAVECRATE_CONFIG_PROFILE"
echo "[run_sandbox] app_root=$APP_ROOT"
echo "[run_sandbox] config=$CONFIG_PATH"
echo "[run_sandbox] logs=$LOGS_DIR"
echo "[run_sandbox] CONTRACT: app config/logs will NOT be read/written from your real user profile dirs (it uses WAVECRATE_CONFIG_HOME)."
if (( WRITE_DB == 1 )); then
  echo "[run_sandbox] Source DB mode: write-enabled (explicit override)."
else
  echo "[run_sandbox] Source DB mode: read-only (default for agent safety)."
fi
if (( ALLOW_USER_LIBRARY_DB_WRITE == 1 )); then
  echo "[run_sandbox] User-library DB writes: explicitly allowed."
else
  echo "[run_sandbox] User-library DB writes: blocked."
fi

if (( WRITE_DB == 0 )); then
  echo "[run_sandbox] DB writes to source trees are blocked by default."
else
  echo "[run_sandbox] DB writes to source trees are enabled for this run."
  if (( ALLOW_USER_LIBRARY_DB_WRITE == 0 )); then
    echo "[run_sandbox] User-library-like source roots are still blocked unless --allow-user-library-db-write is set."
  fi
fi

echo "[run_sandbox] Can still write:"
echo "[run_sandbox]   - sandbox dir: $WAVECRATE_CONFIG_HOME"
echo "[run_sandbox]   - cargo build artifacts: $ROOT_DIR/target (and your rustup/cargo caches)"
if (( TEMP == 1 )); then
  echo "[run_sandbox] Ephemeral mode: sandbox dir will be deleted on exit."
fi

cargo run --release -- "$@"

run_status=$?

exit "$run_status"
