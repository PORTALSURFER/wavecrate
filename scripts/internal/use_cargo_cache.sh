#!/usr/bin/env bash

# Shared local Cargo cache setup for repository scripts.
#
# Scripts should source this file and call `sempal_enable_cargo_cache` before
# invoking Cargo so local runs opportunistically use `sccache` when installed.

set -euo pipefail

sempal_ensure_cargo_available() {
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

sempal_enable_cargo_cache() {
  sempal_ensure_cargo_available

  if [[ "${SEMPAL_DISABLE_SCCACHE:-0}" == "1" ]]; then
    echo "[cargo-cache] sccache disabled by SEMPAL_DISABLE_SCCACHE=1"
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
