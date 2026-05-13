#!/usr/bin/env bash

# Local required CI mirror: run the merge-blocking checks from
# `.github/workflows/ci.yml` that are practical on this platform.

# `--skip-agent-preflight` is useful for agent entrypoint scripts that have already
# run `run_agent_ci_checks.sh` and want to avoid duplicate checks.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/internal/setup_headless_audio.sh
source "$ROOT_DIR/scripts/internal/setup_headless_audio.sh"
wavecrate_setup_headless_audio "ci_local"
# shellcheck source=scripts/internal/use_cargo_cache.sh
source "$ROOT_DIR/scripts/internal/use_cargo_cache.sh"
wavecrate_enable_cargo_cache

SKIP_AGENT_PREFLIGHT=0

while (( $# > 0 )); do
  case "$1" in
    --skip-agent-preflight)
      SKIP_AGENT_PREFLIGHT=1
      shift
      ;;
    -h|--help)
      cat <<'USAGE'
Usage: scripts/ci.sh local [--skip-agent-preflight]

Run the local required CI parity sequence used by this repository.

Options:
  --skip-agent-preflight  Skip ./scripts/internal/agent/run_agent_ci_checks.sh.
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
  echo "[ci_local] scripts/internal/agent/run_agent_ci_checks.sh"
  ./scripts/internal/agent/run_agent_ci_checks.sh
fi

echo "[ci_local] cargo clippy --workspace --all-targets"
cargo clippy --workspace --all-targets

echo "[ci_local] cargo doc -p wavecrate --no-deps (RUSTDOCFLAGS=-D warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc -p wavecrate --no-deps

echo "[ci_local] cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast"
cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast

echo "[ci_local] cargo test --workspace --doc"
cargo test --workspace --doc

echo "[ci_local] OK"
