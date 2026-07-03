#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[release_deps] Ubuntu release dependencies are only needed on Linux."
  exit 0
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "[release_deps] apt-get is required to install Ubuntu release dependencies." >&2
  exit 1
fi

apt_get() {
  if [[ "$(id -u)" == "0" ]]; then
    DEBIAN_FRONTEND=noninteractive apt-get "$@"
  else
    sudo DEBIAN_FRONTEND=noninteractive apt-get "$@"
  fi
}

echo "[release_deps] Installing Ubuntu audio build dependencies for release validation."
apt_get update
apt_get install -y --no-install-recommends pkg-config libasound2-dev

pkg-config --exists alsa
echo "[release_deps] ALSA build dependency available: $(pkg-config --modversion alsa)"
