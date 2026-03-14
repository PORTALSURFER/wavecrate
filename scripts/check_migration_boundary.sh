#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_CORE_DIR="$ROOT_DIR/src/app_core"
ALLOWED_FILE="$APP_CORE_DIR/app_api.rs"
ALLOWED_TRANSITIONAL_FILES=()

if command -v rg >/dev/null 2>&1; then
  mapfile -t matches < <(rg -n "crate::app::" "$APP_CORE_DIR" || true)
else
  mapfile -t matches < <(grep -RIn --include='*.rs' "crate::app::" "$APP_CORE_DIR" || true)
fi

is_test_path() {
  local file="$1"
  [[ "$file" == *"/tests/"* ]] && return 0
  [[ "$file" == *"/tests.rs" ]] && return 0
  [[ "$file" == *"_tests.rs" ]] && return 0
  return 1
}

is_allowed_transitional_path() {
  local file="$1"
  local allowed
  for allowed in "${ALLOWED_TRANSITIONAL_FILES[@]}"; do
    if [[ "$file" == "$allowed" ]]; then
      return 0
    fi
  done
  return 1
}

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
  if is_test_path "$file"; then
    continue
  fi
  if is_allowed_transitional_path "$file"; then
    continue
  fi

  if ((violations == 0)); then
    echo "Migration boundary check failed: direct crate::app references were found outside app_core::app_api."
  fi

  echo " - $match"
  ((violations++))

done

if ((violations > 0)); then
  echo "Allowed app_core migration boundary location: $ALLOWED_FILE"
  exit 1
fi

echo "Migration boundary check passed."
