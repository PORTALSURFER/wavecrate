#!/usr/bin/env bash

# Guards against workflow/toolchain drift by ensuring workflows that install
# Rust derive the toolchain from `rust-toolchain.toml` (not "stable").

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

WORKFLOWS_DIR=".github/workflows"

if [[ ! -d "$WORKFLOWS_DIR" ]]; then
  echo "[toolchain_pinning] Missing $WORKFLOWS_DIR" >&2
  exit 1
fi

files=()
while IFS= read -r f; do files+=("$f"); done < <(ls -1 "$WORKFLOWS_DIR"/*.yml 2>/dev/null || true)

if (( ${#files[@]} == 0 )); then
  echo "[toolchain_pinning] No workflow yml files found under $WORKFLOWS_DIR" >&2
  exit 1
fi

violations=0
for f in "${files[@]}"; do
  if grep -q "dtolnay/rust-toolchain@" "$f"; then
    if ! grep -q "rust-toolchain\\.toml" "$f"; then
      if (( violations == 0 )); then
        echo "[toolchain_pinning] Workflows must derive toolchain from rust-toolchain.toml:" >&2
      fi
      echo " - $f: uses dtolnay/rust-toolchain but does not reference rust-toolchain.toml" >&2
      violations=$((violations + 1))
    fi
  fi

  # Forbid literal stable/beta/nightly toolchain installs (action tags like @stable are OK).
  if grep -Eq '^[[:space:]]*toolchain:[[:space:]]*["'"'"']?(stable|beta|nightly)["'"'"']?[[:space:]]*$' "$f"; then
    if (( violations == 0 )); then
      echo "[toolchain_pinning] Workflows must not set toolchain: stable/beta/nightly literals:" >&2
    fi
    echo " - $f: contains literal toolchain: stable/beta/nightly (use rust-toolchain.toml)" >&2
    violations=$((violations + 1))
  fi

  if grep -Eq 'rustup[[:space:]]+toolchain[[:space:]]+install[[:space:]]+(stable|beta|nightly)\b' "$f"; then
    if (( violations == 0 )); then
      echo "[toolchain_pinning] Workflows must not install toolchain by name:" >&2
    fi
    echo " - $f: installs toolchain by name (use rust-toolchain.toml)" >&2
    violations=$((violations + 1))
  fi

  if grep -q "actions-rs/toolchain" "$f"; then
    if (( violations == 0 )); then
      echo "[toolchain_pinning] Prefer dtolnay/rust-toolchain + rust-toolchain.toml; do not use actions-rs/toolchain:" >&2
    fi
    echo " - $f: uses actions-rs/toolchain" >&2
    violations=$((violations + 1))
  fi
done

if (( violations > 0 )); then
  exit 1
fi

echo "[toolchain_pinning] OK"
exit 0

