#!/usr/bin/env bash

# Shared local Cargo cache setup for repository scripts.
#
# Scripts should source this file and call `wavecrate_enable_cargo_cache` before
# invoking Cargo so local runs use direct `rustc` by default and can opt in to
# `sccache` when needed.

set -euo pipefail

wavecrate_ensure_cargo_available() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi

  if [[ -x "${HOME:-}/.cargo/bin/cargo" ]]; then
    export PATH
    PATH="${HOME}/.cargo/bin:$PATH"
    return 0
  fi

  local cargo_exe=""
  shopt -s nullglob
  for cargo_exe in /mnt/c/Users/*/.cargo/bin/cargo.exe; do
    export PATH
    PATH="$(dirname "$cargo_exe"):$PATH"
    break
  done
  shopt -u nullglob
}

wavecrate_is_sccache_wrapper() {
  local wrapper="${1:-}"
  if [[ -z "$wrapper" ]]; then
    return 1
  fi

  local file_name
  file_name="$(basename "$wrapper")"
  [[ "$file_name" == "sccache" || "$file_name" == "sccache.exe" ]]
}

wavecrate_clear_sccache_wrapper() {
  local reason="$1"

  if wavecrate_is_sccache_wrapper "${RUSTC_WRAPPER:-}" ||
    wavecrate_is_sccache_wrapper "${CARGO_BUILD_RUSTC_WRAPPER:-}"; then
    unset RUSTC_WRAPPER
    unset CARGO_BUILD_RUSTC_WRAPPER
    echo "[cargo-cache] $reason; falling back to direct rustc"
    return 0
  fi

  echo "[cargo-cache] $reason"
}

wavecrate_enable_cargo_cache() {
  wavecrate_ensure_cargo_available

  if [[ "${WAVECRATE_DISABLE_SCCACHE:-0}" == "1" ]]; then
    wavecrate_clear_sccache_wrapper "sccache disabled by WAVECRATE_DISABLE_SCCACHE=1"
    return 0
  fi

  if [[ "${WAVECRATE_ENABLE_SCCACHE:-0}" != "1" ]]; then
    wavecrate_clear_sccache_wrapper "sccache disabled by default"
    return 0
  fi

  if [[ -n "${RUSTC_WRAPPER:-}" ]]; then
    echo "[cargo-cache] keeping existing RUSTC_WRAPPER=$RUSTC_WRAPPER"
    return 0
  fi

  if ! command -v sccache >/dev/null 2>&1; then
    return 0
  fi

  export RUSTC_WRAPPER
  RUSTC_WRAPPER="$(command -v sccache)"
  echo "[cargo-cache] using sccache via RUSTC_WRAPPER=$RUSTC_WRAPPER"
}
