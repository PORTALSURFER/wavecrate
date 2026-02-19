#!/usr/bin/env bash

# Install local git hooks that run mandatory agent preflight checks after pull-like
# repo changes.
#
# Hooks installed:
# - post-merge: runs after pull/merge to refresh MEMORY.md and re-run preflight checks
# - post-checkout: runs after branch/snapshot switches (not after file-only checkouts)
#
# To temporarily disable hook execution, set SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

FORCE=0

usage() {
  cat <<'USAGE'
Usage: scripts/install_agent_preflight_hooks.sh [--force]

Install local git hooks that call scripts/run_agent_preflight.sh after repo-level source
updates.

Options:
  --force  Overwrite existing hooks (a backup is still created if possible).
  -h, --help
           Show this help text.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    --force)
      FORCE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[agent_hook_install] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

HOOK_DIR="$(git rev-parse --git-common-dir)/hooks"

if [[ ! -d "$HOOK_DIR" ]]; then
  echo "[agent_hook_install] Missing hooks directory: $HOOK_DIR" >&2
  exit 1
fi

install_hook() {
  local hook_name="$1"
  local target="$HOOK_DIR/$hook_name"

  if [[ -f "$target" && ! -x "$target" ]]; then
    echo "[agent_hook_install] Existing non-executable hook found: $target" >&2
    exit 1
  fi

  if (( FORCE == 0 )) && [[ -f "$target" ]] && ! grep -q "run_agent_preflight.sh" "$target" 2>/dev/null; then
    echo "[agent_hook_install] Refusing to overwrite existing hook: $target" >&2
    echo "[agent_hook_install] Use --force to replace it." >&2
    exit 1
  fi

  if [[ -f "$target" && (( FORCE == 1 )) ]]; then
    cp "$target" "${target}.pre-agent-backup" 2>/dev/null || true
    echo "[agent_hook_install] Backed up existing hook to ${target}.pre-agent-backup"
  fi

  cat > "$target" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK:-0}" == "1" ]]; then
  exit 0
fi

HOOK_NAME="$(basename "$0")"
if [[ "$HOOK_NAME" == "post-checkout" && "${3:-0}" != "1" ]]; then
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi

preflight_script="$repo_root/scripts/run_agent_preflight.sh"
if [[ -x "$preflight_script" ]]; then
  "$preflight_script" \
    --updater "${AGENT_PREFLIGHT_UPDATER:-Codex}" \
    --memory-max-age-hours "${AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS:-1}" \
    --refresh-memory
else
  echo "[agent_preflight_hook] ERROR: missing $preflight_script" >&2
  exit 1
fi
EOF

  chmod +x "$target"
}

install_hook "post-merge"
install_hook "post-checkout"

echo "[agent_hook_install] Installed hooks:"
echo "[agent_hook_install]   - $HOOK_DIR/post-merge"
echo "[agent_hook_install]   - $HOOK_DIR/post-checkout"
echo "[agent_hook_install] Override with: export SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1"
