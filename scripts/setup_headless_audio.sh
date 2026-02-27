#!/usr/bin/env bash

# Configure a dummy ALSA backend for headless Linux runs so CI logs are not
# flooded with "cannot find card" / "Unknown PCM default" noise.
sempal_setup_headless_audio() {
  local log_prefix="${1:-headless_audio}"

  if [[ "$(uname -s)" != "Linux" ]]; then
    return 0
  fi

  # Keep regular desktop behavior when a display server is present.
  if [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]; then
    return 0
  fi

  # Respect explicit caller overrides.
  if [[ -n "${ALSA_CONFIG_PATH:-}" ]]; then
    echo "[$log_prefix] preserving caller ALSA_CONFIG_PATH=$ALSA_CONFIG_PATH"
    return 0
  fi

  local root_dir
  root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  local dummy_conf="$root_dir/scripts/alsa_headless.conf"
  if [[ ! -f "$dummy_conf" ]]; then
    echo "[$log_prefix] WARN: missing $dummy_conf; using system ALSA defaults" >&2
    return 0
  fi

  export ALSA_CONFIG_PATH="$dummy_conf"
  echo "[$log_prefix] headless Linux detected; using dummy ALSA config ($ALSA_CONFIG_PATH)"
}
