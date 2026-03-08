#!/usr/bin/env bash

# Shared local Cargo cache setup for repository scripts.
#
# Scripts should source this file and call `sempal_enable_cargo_cache` before
# invoking Cargo so local runs opportunistically use `sccache` when installed.

set -euo pipefail

sempal_enable_cargo_cache() {
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
