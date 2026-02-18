#!/usr/bin/env bash

# Creates a small diagnostic bundle to attach to bug reports.
#
# Bundle contents are intentionally limited:
# - latest logs (default: 5)
# - `config.toml` (if present)
# - tool/runtime versions (`rustc`, `cargo`, `git`)
#
# Note: logs and config may contain local paths. Review before sharing.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MAX_LOGS=5
OUT_DIR="dist/bug_bundles"
USE_SANDBOX=0

usage() {
  cat <<'EOF'
Usage: scripts/bug_bundle.sh [--out-dir <dir>] [--logs <n>] [--sandbox]

Creates an archive under <out-dir> containing:
- the newest N log files (default: 5)
- `config.toml` (if present)
- version/system info

Sandbox behavior:
- If `SEMPAL_CONFIG_HOME` is set, it is always used.
- Otherwise, if `<repo>/.sandbox/sempal` exists, this script prefers it.
- Pass `--sandbox` to force using `<repo>/.sandbox/sempal`.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --out-dir)
      OUT_DIR="${2:-}"; shift 2 ;;
    --logs)
      MAX_LOGS="${2:-}"; shift 2 ;;
    --sandbox)
      USE_SANDBOX=1; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[bug_bundle] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
sandbox_config_home="${ROOT_DIR}/.sandbox/sempal"

default_config_base_dir() {
  if [[ -n "${SEMPAL_CONFIG_HOME:-}" ]]; then
    printf "%s" "$SEMPAL_CONFIG_HOME"
    return 0
  fi
  if (( USE_SANDBOX == 1 )) || [[ -d "$sandbox_config_home" ]]; then
    printf "%s" "$sandbox_config_home"
    return 0
  fi
  case "$os_name" in
    darwin)
      printf "%s" "${HOME}/Library/Application Support"
      ;;
    *)
      if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
        printf "%s" "$XDG_CONFIG_HOME"
      else
        printf "%s" "${HOME}/.config"
      fi
      ;;
  esac
}

config_base_dir="$(default_config_base_dir)"
used_sandbox_config_home="false"
if [[ -z "${SEMPAL_CONFIG_HOME:-}" ]] && [[ "$config_base_dir" == "$sandbox_config_home" ]]; then
  used_sandbox_config_home="true"
fi

extract_app_data_dir_from_config() {
  local config_path="$1"
  [[ -f "$config_path" ]] || return 1
  local line
  line="$(rg -m 1 -N '^\\s*app_data_dir\\s*=\\s*' "$config_path" 2>/dev/null || true)"
  [[ -n "$line" ]] || return 1
  local value="${line#*=}"
  value="$(printf "%s" "$value" | sed -E 's/^\\s+//; s/\\s+$//; s/\\s+#.*$//')"
  value="$(printf "%s" "$value" | sed -E 's/^\"(.*)\"$/\\1/; s/^\\x27(.*)\\x27$/\\1/')"
  [[ -n "$value" ]] || return 1
  printf "%s" "$value"
  return 0
}

resolve_app_root_dir() {
  local default_root="${config_base_dir}/.sempal"

  local config_path="${default_root}/config.toml"
  local override_root=""
  if override_root="$(extract_app_data_dir_from_config "$config_path")"; then
    if [[ "$override_root" = /* ]]; then
      printf "%s" "$override_root"
      return 0
    fi
  fi
  printf "%s" "$default_root"
}

app_root="$(resolve_app_root_dir)"
logs_dir="${app_root}/logs"
config_path="${app_root}/config.toml"

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
bundle_root="$(mktemp -d)"
trap 'rm -rf "$bundle_root"' EXIT

bundle_dir="${bundle_root}/sempal-bug-bundle-${timestamp}"
mkdir -p "$bundle_dir"

mkdir -p "${bundle_dir}/meta"
{
  echo "timestamp_utc=${timestamp}"
  echo "repo_root=${ROOT_DIR}"
  echo "config_base_dir=${config_base_dir}"
  echo "preferred_sandbox_config_home=${sandbox_config_home}"
  echo "used_sandbox_config_home=${used_sandbox_config_home}"
  echo "app_root=${app_root}"
  echo "logs_dir=${logs_dir}"
  echo "config_path=${config_path}"
  echo
  echo "rustc_version=$(rustc --version 2>/dev/null || echo 'n/a')"
  echo "cargo_version=$(cargo --version 2>/dev/null || echo 'n/a')"
  echo "git_version=$(git --version 2>/dev/null || echo 'n/a')"
  echo "uname=$(uname -a 2>/dev/null || echo 'n/a')"
} > "${bundle_dir}/meta/info.txt"

if [[ -f "$config_path" ]]; then
  mkdir -p "${bundle_dir}/config"
  cp "$config_path" "${bundle_dir}/config/config.toml"
fi

if [[ -d "$logs_dir" ]]; then
  mkdir -p "${bundle_dir}/logs"
  mapfile -t logs < <(ls -1t "$logs_dir"/*.log 2>/dev/null | head -n "$MAX_LOGS" || true)
  for log_file in "${logs[@]:-}"; do
    [[ -f "$log_file" ]] || continue
    cp "$log_file" "${bundle_dir}/logs/$(basename "$log_file")"
  done
fi

mkdir -p "$OUT_DIR"
archive_base="${OUT_DIR}/sempal-bug-bundle-${timestamp}"

if command -v zip >/dev/null 2>&1; then
  (cd "$bundle_root" && zip -r "${archive_base}.zip" "$(basename "$bundle_dir")" >/dev/null)
  echo "[bug_bundle] wrote ${archive_base}.zip"
elif command -v tar >/dev/null 2>&1; then
  (cd "$bundle_root" && tar -czf "${archive_base}.tar.gz" "$(basename "$bundle_dir")")
  echo "[bug_bundle] wrote ${archive_base}.tar.gz"
else
  echo "[bug_bundle][error] No archive tool found (zip or tar required)." >&2
  exit 1
fi

echo "[bug_bundle] NOTE: logs/config may contain local paths; review before sharing."
