#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_CORE_DIR="$ROOT_DIR/src/app_core"
ALLOWED_FILE="$APP_CORE_DIR/contracts.rs"

mapfile -t matches < <(rg -n "crate::app::" "$APP_CORE_DIR" || true)

if (( ${#matches[@]} == 0 )); then
  echo "Migration boundary check passed: no legacy app references in app_core."
  exit 0
fi

violations=0
for match in "${matches[@]}"; do
  file="${match%%:*}"
  if [[ "$file" == "$ALLOWED_FILE" ]]; then
    continue
  fi

  if ((violations == 0)); then
    echo "Migration boundary check failed: direct crate::app references were found outside app_core::contracts."
  fi

  echo " - $match"
  ((violations++))
done

if ((violations > 0)); then
  echo "Allowed app_core migration boundary location: $ALLOWED_FILE"
  exit 1
fi

echo "Migration boundary check passed."
