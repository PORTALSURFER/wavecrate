#!/usr/bin/env bash

# Select a bounded, toolchain/lockfile-specific Cargo target for supported
# macOS validation. This avoids the unbounded repository-wide debug/deps cache.

set -euo pipefail

wavecrate_use_validation_target() {
  local root_dir="$1"
  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    echo "[validation_target] keeping CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
    return 0
  fi

  local rustc_identity lock_identity target_root target_dir deps_dir
  rustc_identity="$(rustc -vV | awk '/^(release|host):/{print $2}' | tr '\n' '-' | sed 's/-$//')"
  lock_identity="$(shasum -a 256 "$root_dir/Cargo.lock" | awk '{print substr($1, 1, 12)}')"
  target_root="${WAVECRATE_VALIDATION_TARGET_ROOT:-$root_dir/target/agent-validation}"
  target_dir="$target_root/$rustc_identity-$lock_identity"
  deps_dir="$target_dir/debug/deps"

  local max_metadata_bytes="${WAVECRATE_VALIDATION_MAX_DEPS_METADATA_BYTES:-8388608}"
  if [[ -d "$deps_dir" ]]; then
    local metadata_bytes link_count
    metadata_bytes="$(stat -f '%z' "$deps_dir")"
    link_count="$(stat -f '%l' "$deps_dir")"
    if (( metadata_bytes > max_metadata_bytes || link_count >= 60000 )); then
      local quarantine="$target_root/stale-$(date -u +%Y%m%dT%H%M%SZ)-$$"
      mv "$target_dir" "$quarantine"
      echo "[validation_target] rotated pathological Cargo target ($metadata_bytes metadata bytes, $link_count links)"
      echo "[validation_target] quarantined at $quarantine; remove it after diagnostics are no longer needed"
    fi
  fi

  mkdir -p "$target_dir"
  export CARGO_TARGET_DIR="$target_dir"
  echo "[validation_target] CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
}
