#!/usr/bin/env bash

# Select a bounded, toolchain/lockfile-specific Cargo target for supported
# macOS validation. This avoids the unbounded repository-wide debug/deps cache.

set -euo pipefail

wavecrate_use_validation_target() {
  local root_dir="$1"
  local platform="${WAVECRATE_VALIDATION_TEST_PLATFORM:-$(uname -s)}"
  if [[ "$platform" != "Darwin" ]]; then
    return 0
  fi
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    echo "[validation_target] keeping CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
    return 0
  fi

  local rustc_identity lock_identity target_root target_dir deps_dir lease_dir owner_pid
  rustc_identity="$(rustc -vV | awk '/^(release|host):/{print $2}' | tr '\n' '-' | sed 's/-$//')"
  lock_identity="$(shasum -a 256 "$root_dir/Cargo.lock" | awk '{print substr($1, 1, 12)}')"
  target_root="${WAVECRATE_VALIDATION_TARGET_ROOT:-$root_dir/target/agent-validation}"
  target_dir="$target_root/$rustc_identity-$lock_identity"
  deps_dir="$target_dir/debug/deps"
  lease_dir="$target_root/.lock-$rustc_identity-$lock_identity"
  owner_pid="${BASHPID:-$$}"
  mkdir -p "$target_root"

  local missing_owner_attempts=0
  while ! mkdir "$lease_dir" 2>/dev/null; do
    local current_owner=""
    IFS= read -r current_owner 2>/dev/null < "$lease_dir/pid" || current_owner=""
    if [[ "$current_owner" =~ ^[0-9]+$ ]] && kill -0 "$current_owner" 2>/dev/null; then
      missing_owner_attempts=0
      sleep 0.1
      continue
    fi
    if [[ -z "$current_owner" && $missing_owner_attempts -lt 20 ]]; then
      missing_owner_attempts=$((missing_owner_attempts + 1))
      sleep 0.1
      continue
    fi
    rm -f "$lease_dir/pid"
    rmdir "$lease_dir" 2>/dev/null || true
    missing_owner_attempts=0
  done
  printf '%s\n' "$owner_pid" > "$lease_dir/pid"
  export WAVECRATE_VALIDATION_TARGET_LEASE_DIR="$lease_dir"
  export WAVECRATE_VALIDATION_TARGET_LEASE_OWNER="$owner_pid"

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

wavecrate_release_validation_target() {
  local lease_dir="${WAVECRATE_VALIDATION_TARGET_LEASE_DIR:-}"
  local expected_owner="${WAVECRATE_VALIDATION_TARGET_LEASE_OWNER:-}"
  local owner_pid="${BASHPID:-$$}"
  if [[ -z "$lease_dir" || "$expected_owner" != "$owner_pid" ]]; then
    return 0
  fi
  local recorded_owner=""
  if [[ -r "$lease_dir/pid" ]]; then
    recorded_owner="$(<"$lease_dir/pid")"
  fi
  if [[ "$recorded_owner" == "$owner_pid" ]]; then
    rm -f "$lease_dir/pid"
    rmdir "$lease_dir" 2>/dev/null || true
  fi
  unset WAVECRATE_VALIDATION_TARGET_LEASE_DIR
  unset WAVECRATE_VALIDATION_TARGET_LEASE_OWNER
}
