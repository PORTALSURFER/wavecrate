#!/usr/bin/env bash

# Select a bounded, toolchain/lockfile-specific Cargo target for supported
# macOS validation. This avoids the unbounded repository-wide debug/deps cache.

set -euo pipefail

wavecrate_process_identity() {
  local pid="$1"
  local process_line state identity
  process_line="$(ps -p "$pid" -o state= -o lstart= 2>/dev/null || true)"
  process_line="$(printf '%s' "$process_line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ -n "$process_line" ]] || return 1
  state="${process_line%%[[:space:]]*}"
  [[ "$state" != Z* ]] || return 1
  identity="${process_line#"$state"}"
  identity="$(printf '%s' "$identity" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ -n "$identity" ]] || return 1
  printf '%s\n' "$identity"
}

wavecrate_directory_metadata() {
  python3 - "$1" <<'PY'
import os
import sys

metadata = os.stat(sys.argv[1])
print(metadata.st_size, metadata.st_nlink)
PY
}

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

  local rustc_identity lock_identity target_root target_dir deps_dir lease_dir owner_pid owner_identity
  rustc_identity="$(rustc -vV | awk '/^(release|host):/{print $2}' | tr '\n' '-' | sed 's/-$//')"
  lock_identity="$(shasum -a 256 "$root_dir/Cargo.lock" | awk '{print substr($1, 1, 12)}')"
  target_root="${WAVECRATE_VALIDATION_TARGET_ROOT:-$root_dir/target/agent-validation}"
  target_dir="$target_root/$rustc_identity-$lock_identity"
  deps_dir="$target_dir/debug/deps"
  lease_dir="$target_root/.lock-$rustc_identity-$lock_identity"
  owner_pid="${BASHPID:-$$}"
  owner_identity="$(wavecrate_process_identity "$owner_pid")" || {
    echo "[validation_target] unable to identify lease owner process $owner_pid" >&2
    return 1
  }
  mkdir -p "$target_root"

  local missing_owner_attempts=0
  local lease_wait_seconds="${WAVECRATE_VALIDATION_LEASE_WAIT_SECONDS:-480}"
  [[ "$lease_wait_seconds" =~ ^[1-9][0-9]*$ ]] || {
    echo "[validation_target] WAVECRATE_VALIDATION_LEASE_WAIT_SECONDS must be a positive integer" >&2
    return 2
  }
  local lease_deadline=$(( $(date +%s) + lease_wait_seconds ))
  while ! mkdir "$lease_dir" 2>/dev/null; do
    local current_owner="" recorded_identity="" current_identity=""
    IFS=$'\t' read -r current_owner recorded_identity 2>/dev/null < "$lease_dir/pid" || current_owner=""
    if [[ "$current_owner" =~ ^[0-9]+$ ]] \
      && [[ -n "$recorded_identity" ]] \
      && current_identity="$(wavecrate_process_identity "$current_owner")" \
      && [[ "$current_identity" == "$recorded_identity" ]]; then
      missing_owner_attempts=0
      if (( $(date +%s) >= lease_deadline )); then
        echo "[validation_target] timed out after ${lease_wait_seconds}s waiting for active owner $current_owner ($lease_dir)" >&2
        return 124
      fi
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
  printf '%s\t%s\n' "$owner_pid" "$owner_identity" > "$lease_dir/pid"
  export WAVECRATE_VALIDATION_TARGET_LEASE_DIR="$lease_dir"
  export WAVECRATE_VALIDATION_TARGET_LEASE_OWNER="$owner_pid"
  export WAVECRATE_VALIDATION_TARGET_LEASE_IDENTITY="$owner_identity"

  local max_metadata_bytes="${WAVECRATE_VALIDATION_MAX_DEPS_METADATA_BYTES:-8388608}"
  if [[ -d "$deps_dir" ]]; then
    local metadata metadata_bytes link_count
    metadata="$(wavecrate_directory_metadata "$deps_dir")"
    read -r metadata_bytes link_count <<< "$metadata"
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
  local expected_identity="${WAVECRATE_VALIDATION_TARGET_LEASE_IDENTITY:-}"
  local owner_pid="${BASHPID:-$$}"
  if [[ -z "$lease_dir" || "$expected_owner" != "$owner_pid" || -z "$expected_identity" ]]; then
    return 0
  fi
  local recorded_owner="" recorded_identity=""
  if [[ -r "$lease_dir/pid" ]]; then
    IFS=$'\t' read -r recorded_owner recorded_identity < "$lease_dir/pid" || true
  fi
  if [[ "$recorded_owner" == "$owner_pid" && "$recorded_identity" == "$expected_identity" ]]; then
    rm -f "$lease_dir/pid"
    rmdir "$lease_dir" 2>/dev/null || true
  fi
  unset WAVECRATE_VALIDATION_TARGET_LEASE_DIR
  unset WAVECRATE_VALIDATION_TARGET_LEASE_OWNER
  unset WAVECRATE_VALIDATION_TARGET_LEASE_IDENTITY
}
