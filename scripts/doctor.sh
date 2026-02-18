#!/usr/bin/env bash

# Environment sanity checks for local development and agent runs.
# Prints common pitfalls called out in README and where to find logs.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0
warnings=0

info() { printf "%s\n" "[doctor] $*"; }
warn() { printf "%s\n" "[doctor][warn] $*" >&2; warnings=$((warnings + 1)); }
err() { printf "%s\n" "[doctor][error] $*" >&2; failures=$((failures + 1)); }

os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
is_wsl=0
if [[ -r /proc/version ]] && rg -q -i "microsoft" /proc/version 2>/dev/null; then
  is_wsl=1
fi

info "Repo: $ROOT_DIR"
info "OS: ${os_name}${is_wsl:+ (wsl)}"

info "Checking Git LFS..."
if command -v git >/dev/null 2>&1; then
  if git lfs version >/dev/null 2>&1; then
    info "Git LFS: present"
  else
    warn "Git LFS: missing (install git-lfs if you see checkout/build issues with large assets)"
  fi
else
  warn "git not found on PATH"
fi

info "Checking SEMPAL_NATIVE_FONT_PATH..."
if [[ -n "${SEMPAL_NATIVE_FONT_PATH:-}" ]]; then
  if [[ -f "$SEMPAL_NATIVE_FONT_PATH" ]]; then
    info "SEMPAL_NATIVE_FONT_PATH: OK ($SEMPAL_NATIVE_FONT_PATH)"
  else
    err "SEMPAL_NATIVE_FONT_PATH is set but not a file: $SEMPAL_NATIVE_FONT_PATH"
  fi
else
  info "SEMPAL_NATIVE_FONT_PATH: not set (OK)"
fi

info "Checking CPAL_ASIO_DIR (Windows ASIO builds)..."
if [[ -n "${CPAL_ASIO_DIR:-}" ]]; then
  if [[ -d "$CPAL_ASIO_DIR" ]]; then
    if [[ -d "$CPAL_ASIO_DIR/host" && -d "$CPAL_ASIO_DIR/common" ]]; then
      info "CPAL_ASIO_DIR: looks like an ASIO SDK root ($CPAL_ASIO_DIR)"
    else
      err "CPAL_ASIO_DIR exists but doesn't look like ASIO SDK root (expected host/ and common/): $CPAL_ASIO_DIR"
    fi
  else
    err "CPAL_ASIO_DIR is set but not a directory: $CPAL_ASIO_DIR"
  fi
else
  info "CPAL_ASIO_DIR: not set (OK unless building Windows ASIO support)"
fi

info "Expected log locations:"
info "  Linux:   \$HOME/.config/.sempal/logs"
info "  macOS:   \$HOME/Library/Application Support/.sempal/logs"
info "  Windows: %APPDATA%\\\\.sempal\\\\logs"
if (( is_wsl == 1 )); then
  info "  WSL hint: /mnt/c/Users/<you>/AppData/Roaming/.sempal/logs"
fi

if (( failures > 0 )); then
  info "Result: FAIL ($failures errors, $warnings warnings)"
  exit 1
fi

info "Result: OK ($warnings warnings)"
exit 0

