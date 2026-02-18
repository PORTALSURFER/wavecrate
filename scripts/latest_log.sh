#!/usr/bin/env bash

# Prints the resolved log directory, the newest log file, and a tail snippet.
#
# Resolution order for the `.sempal` root:
# 1) `SEMPAL_CONFIG_HOME` (config base override, if set)
# 2) OS default config base (`XDG_CONFIG_HOME` or `~/.config`, macOS app-support, Windows APPDATA via WSL hint)
# 3) `app_data_dir` in `<app_root>/config.toml` (absolute path expected; overrides `.sempal` root)
#
# This is best-effort and intended for quick diagnostics (humans + agents).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

LINES=200
USE_SANDBOX=0

usage() {
  cat <<'EOF'
Usage: scripts/latest_log.sh [--lines <n>] [--sandbox]

Prints:
- resolved `.sempal` root (best-effort)
- resolved logs dir
- newest `*.log` file under logs dir (if any)
- tail snippet from that newest log (default: 200 lines)

Sandbox behavior:
- If `SEMPAL_CONFIG_HOME` is set, it is always used.
- Otherwise, if `<repo>/.sandbox/sempal` exists, this script prefers it.
- Pass `--sandbox` to force using `<repo>/.sandbox/sempal`.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --lines)
      LINES="${2:-}"; shift 2 ;;
    --sandbox)
      USE_SANDBOX=1; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[latest_log] Unknown argument: $1" >&2
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

  # TOML: app_data_dir = "/abs/path"
  local line
  line="$(rg -m 1 -N '^\\s*app_data_dir\\s*=\\s*' "$config_path" 2>/dev/null || true)"
  [[ -n "$line" ]] || return 1

  # Strip key/value prefix.
  local value="${line#*=}"
  value="$(printf "%s" "$value" | sed -E 's/^\\s+//; s/\\s+$//')"

  # Remove trailing inline comments.
  value="$(printf "%s" "$value" | sed -E 's/\\s+#.*$//')"

  # Remove surrounding quotes if present.
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
    echo "[latest_log][warn] app_data_dir in $config_path is not an absolute path; ignoring: $override_root" >&2
  fi

  printf "%s" "$default_root"
}

app_root="$(resolve_app_root_dir)"
logs_dir="${app_root}/logs"

echo "[latest_log] config_base_dir=$config_base_dir"
echo "[latest_log] preferred_sandbox_config_home=$sandbox_config_home"
echo "[latest_log] used_sandbox_config_home=$used_sandbox_config_home"
echo "[latest_log] app_root=$app_root"
echo "[latest_log] logs_dir=$logs_dir"

if [[ ! -d "$logs_dir" ]]; then
  echo "[latest_log][error] logs dir does not exist: $logs_dir" >&2
  exit 1
fi

newest_log="$(ls -1t "$logs_dir"/*.log 2>/dev/null | head -n 1 || true)"
if [[ -z "$newest_log" ]]; then
  echo "[latest_log] No .log files found under $logs_dir"
  exit 0
fi

echo "[latest_log] newest_log=$newest_log"
echo "[latest_log] tail_lines=$LINES"
tail -n "$LINES" "$newest_log"
