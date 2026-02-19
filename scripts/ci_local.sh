#!/usr/bin/env bash

# Local CI mirror: run the same checks as `.github/workflows/ci.yml`.
# This script is intentionally simple so agents and humans can rely on it.

# `--skip-agent-preflight` is useful for agent entrypoint scripts that have already
# run `run_agent_ci_checks.sh` and want to avoid duplicate checks.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

SKIP_AGENT_PREFLIGHT=0
MEMORY_MAX_AGE_HOURS="${AGENT_CI_MEMORY_MAX_AGE_HOURS:-24}"
REQUIRED_UPDATER="${AGENT_CI_REQUIRED_UPDATER:-}"

while (( $# > 0 )); do
  case "$1" in
    --skip-agent-preflight)
      SKIP_AGENT_PREFLIGHT=1
      shift
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: scripts/ci_local.sh [--skip-agent-preflight]

Run the local CI sequence used by this repository.

Options:
  --skip-agent-preflight  Skip ./scripts/run_agent_ci_checks.sh.
  -h, --help             Show this help text.
USAGE
      exit 0
      ;;
    *)
      echo "[ci_local] Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

echo "[ci_local] cargo fmt --all -- --check"
cargo fmt --all -- --check

if (( SKIP_AGENT_PREFLIGHT == 0 )); then
  echo "[ci_local] scripts/run_agent_ci_checks.sh"
  if [[ -n "$REQUIRED_UPDATER" ]]; then
    ./scripts/run_agent_ci_checks.sh \
      --required-updater "$REQUIRED_UPDATER" \
      --memory-max-age-hours "$MEMORY_MAX_AGE_HOURS"
  else
    ./scripts/run_agent_ci_checks.sh \
      --memory-max-age-hours "$MEMORY_MAX_AGE_HOURS"
  fi
fi

echo "[ci_local] cargo clippy --all-targets"
cargo clippy --all-targets

echo "[ci_local] cargo doc -p sempal --no-deps (RUSTDOCFLAGS=-D warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc -p sempal --no-deps

echo "[ci_local] cargo test --all-targets"
cargo test --all-targets

echo "[ci_local] scripts/run_perf_guard.sh"
./scripts/run_perf_guard.sh

echo "[ci_local] OK"
