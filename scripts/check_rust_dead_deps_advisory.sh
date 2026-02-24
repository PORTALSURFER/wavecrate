#!/usr/bin/env bash

# Advisory sweep for unused Rust dependencies and dead code paths.
#
# This script is intentionally non-blocking by default so teams can observe
# signal quality before promoting it to a strict CI gate.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/check_rust_dead_deps_advisory.sh [options]

Run advisory dead-dependency and unused-code checks.

Options:
  --advisory                Non-blocking mode (default).
  --strict                  Exit non-zero when findings are detected.
  --run-udeps               Run cargo-udeps when available.
  --skip-udeps              Skip cargo-udeps checks.
  --install-missing         Install missing cargo subcommands before running.
  --no-install-missing      Never install missing cargo subcommands (default).
  --udeps-toolchain <name>  Toolchain for cargo-udeps (default: nightly).
  --report <path>           Write full check output to this path.
  -h, --help                Show this help text.

Environment:
  SEMPAL_DEAD_SWEEP_STRICT=1|true|yes|on
  SEMPAL_DEAD_SWEEP_RUN_UDEPS=1|true|yes|on
  SEMPAL_DEAD_SWEEP_INSTALL_MISSING=1|true|yes|on
  SEMPAL_DEAD_SWEEP_UDEPS_TOOLCHAIN=<toolchain>
  SEMPAL_DEAD_SWEEP_REPORT_PATH=<path>
USAGE
}

is_truthy() {
  local value="${1,,}"
  [[ "$value" == "1" || "$value" == "true" || "$value" == "yes" || "$value" == "on" ]]
}

to_bool() {
  local value="${1:-0}"
  if is_truthy "$value"; then
    echo "1"
  else
    echo "0"
  fi
}

STRICT_MODE="$(to_bool "${SEMPAL_DEAD_SWEEP_STRICT:-0}")"
RUN_UDEPS="$(to_bool "${SEMPAL_DEAD_SWEEP_RUN_UDEPS:-0}")"
INSTALL_MISSING="$(to_bool "${SEMPAL_DEAD_SWEEP_INSTALL_MISSING:-0}")"
UDEPS_TOOLCHAIN="${SEMPAL_DEAD_SWEEP_UDEPS_TOOLCHAIN:-nightly}"
REPORT_PATH="${SEMPAL_DEAD_SWEEP_REPORT_PATH:-}"

while (( $# > 0 )); do
  case "$1" in
    --advisory)
      STRICT_MODE=0
      shift
      ;;
    --strict)
      STRICT_MODE=1
      shift
      ;;
    --run-udeps)
      RUN_UDEPS=1
      shift
      ;;
    --skip-udeps)
      RUN_UDEPS=0
      shift
      ;;
    --install-missing)
      INSTALL_MISSING=1
      shift
      ;;
    --no-install-missing)
      INSTALL_MISSING=0
      shift
      ;;
    --udeps-toolchain)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[dead_sweep] --udeps-toolchain requires a value." >&2
        usage >&2
        exit 2
      fi
      UDEPS_TOOLCHAIN="${2:-}"
      shift 2
      ;;
    --report)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        echo "[dead_sweep] --report requires a value." >&2
        usage >&2
        exit 2
      fi
      REPORT_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[dead_sweep] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

REPORT_TMP="$(mktemp)"
trap 'rm -f "$REPORT_TMP"' EXIT

append_report() {
  printf '%s\n' "$1" >>"$REPORT_TMP"
}

ensure_subcommand_installed() {
  local binary_name="$1"
  local install_package="$2"
  if command -v "$binary_name" >/dev/null 2>&1; then
    return 0
  fi
  if (( INSTALL_MISSING == 0 )); then
    echo "[dead_sweep] SKIP: $binary_name not found (set --install-missing to auto-install)."
    append_report "[dead_sweep] SKIP: $binary_name not found."
    return 1
  fi
  echo "[dead_sweep] Installing missing tool: $install_package"
  if cargo install --locked "$install_package"; then
    return 0
  fi
  echo "[dead_sweep] WARN: failed to install $install_package"
  append_report "[dead_sweep] WARN: failed to install $install_package"
  return 1
}

run_check() {
  local label="$1"
  shift

  local output_file
  output_file="$(mktemp)"
  set +e
  "$@" >"$output_file" 2>&1
  local exit_code=$?
  set -e

  append_report "### $label"
  cat "$output_file" >>"$REPORT_TMP"
  append_report ""

  if (( exit_code == 0 )); then
    echo "[dead_sweep] PASS: $label"
    rm -f "$output_file"
    return 0
  fi

  echo "[dead_sweep] FINDING: $label (exit $exit_code)"
  sed -n '1,120p' "$output_file"
  rm -f "$output_file"
  return "$exit_code"
}

findings=0

if ensure_subcommand_installed "cargo-machete" "cargo-machete"; then
  if ! run_check "cargo machete" cargo machete; then
    findings=$((findings + 1))
  fi
fi

if (( RUN_UDEPS == 1 )); then
  if ensure_subcommand_installed "cargo-udeps" "cargo-udeps"; then
    if ! run_check \
      "cargo +${UDEPS_TOOLCHAIN} udeps --all-targets" \
      cargo "+${UDEPS_TOOLCHAIN}" udeps --all-targets; then
      findings=$((findings + 1))
    fi
  fi
else
  echo "[dead_sweep] INFO: cargo-udeps check skipped (enable with --run-udeps)."
  append_report "[dead_sweep] INFO: cargo-udeps check skipped."
fi

if [[ -n "$REPORT_PATH" ]]; then
  mkdir -p "$(dirname "$REPORT_PATH")"
  cp "$REPORT_TMP" "$REPORT_PATH"
  echo "[dead_sweep] Wrote report: $REPORT_PATH"
fi

if (( findings > 0 )); then
  if (( STRICT_MODE == 1 )); then
    echo "[dead_sweep] FAILED: $findings finding(s) in strict mode."
    exit 1
  fi
  echo "[dead_sweep] ADVISORY: $findings finding(s) detected (non-blocking mode)."
  exit 0
fi

echo "[dead_sweep] OK"
exit 0
