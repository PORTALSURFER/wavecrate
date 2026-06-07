#!/usr/bin/env bash

# Enforces the native-app app-chrome versus domain module boundary documented in
# docs/TARGET.md.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: scripts/internal/check/check_native_app_boundary.sh

Fails when native-app domain modules import app_chrome or when ambiguous root
native-app module names are introduced.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[native_app_boundary] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

violations=()
domain_roots=(audio library_browser metadata waveform workflows)
ambiguous_modules=(browser context_menu widgets)

is_domain_native_app_path() {
  local path="$1"
  local root
  for root in "${domain_roots[@]}"; do
    [[ "$path" == "src/native_app/$root.rs" ]] && return 0
    [[ "$path" == src/native_app/"$root"/* ]] && return 0
  done
  return 1
}

is_test_or_support_path() {
  local path="$1"
  [[ "$path" == "src/native_app/test_support.rs" ]] && return 0
  [[ "$path" == "src/native_app/tests.rs" ]] && return 0
  [[ "$path" == src/native_app/tests/* ]] && return 0
  [[ "$path" == */tests/* ]] && return 0
  [[ "$path" == *_tests.rs ]] && return 0
  return 1
}

while IFS= read -r -d '' file; do
  repo_path="${file#./}"
  is_domain_native_app_path "$repo_path" || continue
  is_test_or_support_path "$repo_path" && continue

  while IFS=: read -r line_number line_text; do
    [[ "$line_text" =~ ^[[:space:]]*// ]] && continue
    violations+=("$repo_path:$line_number: domain modules must not import app_chrome: ${line_text#"${line_text%%[![:space:]]*}"}")
  done < <(grep -nE '\bcrate::native_app::app_chrome\b' "$file" || true)
done < <(find ./src/native_app -type f -name '*.rs' -print0)

for module_name in "${ambiguous_modules[@]}"; do
  if [[ -f "src/native_app/$module_name.rs" ]]; then
    violations+=("src/native_app/$module_name.rs: ambiguous root native-app module; move feature-specific code under its owner, e.g. app_chrome or library_browser")
  fi

  if [[ -f src/native_app.rs ]]; then
    while IFS=: read -r line_number _line_text; do
      violations+=("src/native_app.rs:$line_number: ambiguous root native-app module declaration \`mod $module_name;\`")
    done < <(grep -nE "^[[:space:]]*(pub([[:space:]]*\\([^)]*\\))?[[:space:]]+)?mod[[:space:]]+$module_name[[:space:]]*;" src/native_app.rs || true)
  fi
done

if (( ${#violations[@]} > 0 )); then
  echo "[native_app_boundary] Native app boundary violations detected:"
  echo "[native_app_boundary] app_chrome is the view-composition layer; product/domain modules must depend on messages, view models, or domain APIs instead."
  echo "[native_app_boundary] Root native-app module names must describe durable ownership, not generic widgets. See docs/TARGET.md native app module map."
  printf '%s\n' "${violations[@]}" | sort | sed 's/^/ - /'
  exit 1
fi

echo "[native_app_boundary] OK"
exit 0
