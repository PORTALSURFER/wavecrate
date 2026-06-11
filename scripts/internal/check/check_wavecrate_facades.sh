#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/check/check_wavecrate_facades.sh

Fails when selected Wavecrate/app-core facades grow or when test/legacy
crossings bypass their owners.
USAGE
}

if (( $# > 0 )); then
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[wavecrate_facades] Unknown argument: $1" >&2
      exit 2
      ;;
  esac
fi

violations=()

check_facade() {
  local path="$1"
  local max_lines="$2"
  local max_exports="$3"
  local max_modules="$4"
  local owner="$5"
  local reason="$6"

  if [[ ! -f "$path" ]]; then
    violations+=("$path: guarded facade is missing ($owner)")
    return
  fi

  local lines exports modules
  lines="$(wc -l < "$path" | tr -d ' ')"
  exports="$(grep -Ec '^[[:space:]]*pub([[:space:]]*\([^)]*\))?[[:space:]]+(use|type)\b' "$path" || true)"
  modules="$(grep -Ec '^[[:space:]]*pub([[:space:]]*\([^)]*\))?[[:space:]]+mod[[:space:]]+[[:alnum:]_]+[[:space:]]*(;|\{)' "$path" || true)"

  if (( lines > max_lines )); then
    violations+=("$path: $lines lines exceeds facade budget $max_lines ($owner; $reason)")
  fi
  if (( exports > max_exports )); then
    violations+=("$path: $exports root exports exceeds budget $max_exports ($owner; $reason)")
  fi
  if (( modules > max_modules )); then
    violations+=("$path: $modules public modules exceeds budget $max_modules ($owner; $reason)")
  fi
}

is_test_path() {
  local path="$1"
  [[ "$path" == */tests.rs || "$path" == *"/tests/"* || "$path" == *_tests.rs ]]
}

check_facade "src/native_app/test_support.rs" 80 9 0 "OPT-541" \
  "native test fixtures must stay split by focused support module"
check_facade "src/native_app/sample_library/folder_browser.rs" 180 13 6 "OPT-529" \
  "folder browser root remains a facade over owned browsing modules"
check_facade "src/app_core/app_api.rs" 180 24 4 "OPT-537/OPT-538" \
  "legacy crossings are an audited migration surface"
check_facade "src/app_core/actions/mod.rs" 260 66 1 "OPT-539" \
  "action catalog/type exports must shrink by domain instead of growing at the root"

while IFS= read -r -d '' file; do
  repo_path="${file#./}"
  if is_test_path "$repo_path"; then
    continue
  fi
  case "$repo_path" in
    src/native_app/test_support.rs|src/native_app/test_support/*) continue ;;
  esac
  while IFS=: read -r line_number line_text; do
    [[ -z "${line_number:-}" ]] && continue
    violations+=("$repo_path:$line_number: production native-app code must not import test_support: ${line_text#"${line_text%%[![:space:]]*}"}")
  done < <(grep -nE '\b(crate::native_app::test_support|super::test_support)\b' "$repo_path" || true)
done < <(find src/native_app -type f -name '*.rs' -print0)

while IFS= read -r -d '' file; do
  repo_path="${file#./}"
  if is_test_path "$repo_path"; then
    continue
  fi
  [[ "$repo_path" == "src/app_core/app_api.rs" ]] && continue
  while IFS=: read -r line_number line_text; do
    [[ -z "${line_number:-}" ]] && continue
    [[ "$line_text" =~ ^[[:space:]]*// ]] && continue
    violations+=("$repo_path:$line_number: app-core legacy crossings must go through app_core::app_api: ${line_text#"${line_text%%[![:space:]]*}"}")
  done < <(grep -nE '\bcrate::app::' "$repo_path" || true)
done < <(find src/app_core -type f -name '*.rs' -print0)

if (( ${#violations[@]} > 0 )); then
  echo "[wavecrate_facades] Facade guardrail violations detected:"
  echo "[wavecrate_facades] Keep root facades small, route legacy app crossings through app_core::app_api, and keep test_support out of production imports."
  printf ' - %s\n' "${violations[@]}" | sort
  exit 1
fi

echo "[wavecrate_facades] OK"
