#!/usr/bin/env bash

# Required deterministic guardrails for Radiant's non-blocking app
# architecture. Static scans and controlled diagnostics are hard gates; flaky
# timing benchmarks stay outside this lane.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"
# shellcheck source=scripts/internal/use_cargo_cache.sh
source "$ROOT_DIR/scripts/internal/use_cargo_cache.sh"
wavecrate_enable_cargo_cache

usage() {
  cat <<'EOF'
Usage: scripts/internal/check/check_non_blocking_architecture.sh

Run required Radiant and Wavecrate non-blocking architecture guardrails.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[non_blocking_architecture] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

run_step() {
  local label="$1"
  shift
  echo "[non_blocking_architecture] $label"
  "$@"
}

run_step "Radiant synthetic blocking-token fixture" \
  cargo test --manifest-path vendor/radiant/Cargo.toml guardrail_reports_file_line_and_guidance_for_blocking_tokens

run_step "Radiant app/runtime/example guardrails" \
  cargo test --manifest-path vendor/radiant/Cargo.toml --test generic_surface_guardrails source_quality::runtime::commands_and_app

run_step "Wavecrate app-facing blocking guardrail" \
  cargo test -p wavecrate --no-default-features native_app_ui_update_paths_do_not_call_blocking_business_apis

run_step "Wavecrate strict slow-handler diagnostics harness" \
  cargo test -p wavecrate --no-default-features rapid_navigation_harness_keeps_ui_responsive_while_business_work_is_slow

echo "[non_blocking_architecture] OK"
