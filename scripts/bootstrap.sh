#!/usr/bin/env bash

# Bootstrap a known-good local dev environment (humans + agents).
#
# - Installs the pinned Rust toolchain from rust-toolchain.toml
# - Ensures rustfmt/clippy are available
# - Checks git-lfs and python3
# - Prints next-step commands

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[bootstrap] repo: $ROOT_DIR"

if ! command -v git >/dev/null 2>&1; then
  echo "[bootstrap] ERROR: git not found on PATH" >&2
  exit 1
fi

if command -v rg >/dev/null 2>&1; then
  echo "[bootstrap] rg: OK"
else
  echo "[bootstrap] rg: MISSING (recommended; several repo checks use it)" >&2
  echo "[bootstrap]   Install ripgrep (rg). Examples:" >&2
  echo "[bootstrap]     macOS: brew install ripgrep" >&2
  echo "[bootstrap]     Ubuntu/Debian: sudo apt-get install ripgrep" >&2
  echo "[bootstrap]     Windows: winget install BurntSushi.ripgrep.MSVC" >&2
fi

if command -v git-lfs >/dev/null 2>&1 || (git lfs version >/dev/null 2>&1); then
  echo "[bootstrap] git-lfs: OK"
  # Ensure hooks are configured locally (best effort).
  git lfs install --local >/dev/null 2>&1 || true
else
  echo "[bootstrap] git-lfs: MISSING (recommended)"
  echo "[bootstrap]   Install git-lfs and run: git lfs install --local" >&2
fi

if command -v python3 >/dev/null 2>&1; then
  py_ver="$(python3 -c 'import sys; print(".".join(map(str, sys.version_info[:3])))' 2>/dev/null || echo "unknown")"
  echo "[bootstrap] python3: OK ($py_ver)"
else
  echo "[bootstrap] python3: MISSING (recommended; used by some tooling/scripts)" >&2
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "[bootstrap] ERROR: rustup not found on PATH" >&2
  echo "[bootstrap]   Install rustup from https://rustup.rs/ and re-run this script." >&2
  exit 1
fi

if [[ ! -f rust-toolchain.toml ]]; then
  echo "[bootstrap] ERROR: rust-toolchain.toml not found at repo root" >&2
  exit 1
fi

read_toolchain_channel() {
  # Prefer Python 3.11+ tomllib, but fall back to a simple regex for the single
  # `toolchain.channel = "..."` value this repo uses.
  if command -v python3 >/dev/null 2>&1; then
    if python3 - <<'PY' >/dev/null 2>&1
import sys
sys.exit(0 if sys.version_info >= (3, 11) else 1)
PY
    then
      python3 - <<'PY'
import tomllib
from pathlib import Path
data = tomllib.loads(Path("rust-toolchain.toml").read_text(encoding="utf-8"))
print(data.get("toolchain", {}).get("channel", "stable"))
PY
      return 0
    fi
  fi

  # Minimal fallback: extract first `channel = "..."` line.
  sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' rust-toolchain.toml | head -n 1
}

channel="$(read_toolchain_channel | tr -d '[:space:]')"
if [[ -z "$channel" ]]; then
  channel="stable"
fi

echo "[bootstrap] rust toolchain (pinned): $channel"

echo "[bootstrap] rustup toolchain install $channel"
rustup toolchain install "$channel" --profile minimal

echo "[bootstrap] rustup component add rustfmt clippy --toolchain $channel"
rustup component add rustfmt clippy --toolchain "$channel"

echo
echo "[bootstrap] Next steps:"
echo "  - Environment sanity:   bash scripts/doctor.sh"
echo "  - CI parity checks:     bash scripts/ci_local.sh"
echo "  - Safe local run:       bash scripts/run_sandbox.sh --"
