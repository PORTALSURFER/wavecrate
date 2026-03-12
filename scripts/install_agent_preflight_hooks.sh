#!/usr/bin/env bash

# Install local git hooks that keep sempal and the radiant submodule on their
# shared development branches and rerun lightweight preflight checks after
# branch/source updates.
#
# Hooks installed for sempal:
# - post-merge / post-checkout: rerun agent preflight
# - pre-commit / pre-push: fail unless sempal uses local `next` tracking `origin/next`
#
# Hooks installed for vendor/radiant:
# - post-merge / post-checkout / pre-commit / pre-push: fail unless radiant uses
#   local `next` tracking `origin/next`
#
# To temporarily disable hook execution, set SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

FORCE=0

usage() {
  cat <<'USAGE'
Usage: scripts/install_agent_preflight_hooks.sh [--force]

Install local git hooks that keep sempal and vendor/radiant on their shared
development branches and rerun agent preflight checks after repo-level source
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

ensure_hook_dir() {
  local hook_dir="$1"
  if [[ ! -d "$hook_dir" ]]; then
    echo "[agent_hook_install] Missing hooks directory: $hook_dir" >&2
    exit 1
  fi
}

write_hook() {
  local hook_dir="$1"
  local hook_name="$2"
  local sentinel="$3"
  local target="$hook_dir/$hook_name"

  if [[ -f "$target" && ! -x "$target" ]]; then
    echo "[agent_hook_install] Existing non-executable hook found: $target" >&2
    exit 1
  fi

  if (( FORCE == 0 )) && [[ -f "$target" ]] && ! grep -q "$sentinel" "$target" 2>/dev/null; then
    echo "[agent_hook_install] Refusing to overwrite existing hook: $target" >&2
    echo "[agent_hook_install] Use --force to replace it." >&2
    exit 1
  fi

  if [[ -f "$target" && (( FORCE == 1 )) ]]; then
    cp "$target" "${target}.pre-agent-backup" 2>/dev/null || true
    echo "[agent_hook_install] Backed up existing hook to ${target}.pre-agent-backup"
  fi

  cat > "$target"
  chmod +x "$target"
}

ROOT_HOOK_DIR="$(git rev-parse --git-common-dir)/hooks"
ensure_hook_dir "$ROOT_HOOK_DIR"

write_hook "$ROOT_HOOK_DIR" "post-merge" "run_agent_preflight.sh" <<'EOF'
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

write_hook "$ROOT_HOOK_DIR" "post-checkout" "run_agent_preflight.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK:-0}" == "1" ]]; then
  exit 0
fi

if [[ "${3:-0}" != "1" ]]; then
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

write_hook "$ROOT_HOOK_DIR" "pre-commit" "check_next_branch.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK:-0}" == "1" ]]; then
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi

branch_guard="$repo_root/scripts/check_next_branch.sh"
if [[ -x "$branch_guard" ]]; then
  "$branch_guard"
else
  echo "[branch_guard] ERROR: missing $branch_guard" >&2
  exit 1
fi
EOF

write_hook "$ROOT_HOOK_DIR" "pre-push" "check_next_branch.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK:-0}" == "1" ]]; then
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$repo_root" ]]; then
  exit 0
fi

branch_guard="$repo_root/scripts/check_next_branch.sh"
if [[ -x "$branch_guard" ]]; then
  "$branch_guard"
else
  echo "[branch_guard] ERROR: missing $branch_guard" >&2
  exit 1
fi
EOF

RADIANT_DIR="$ROOT_DIR/vendor/radiant"
RADIANT_HOOK_DIR=""
if git -C "$RADIANT_DIR" rev-parse --git-common-dir >/dev/null 2>&1; then
  RADIANT_HOOK_DIR="$(git -C "$RADIANT_DIR" rev-parse --git-common-dir)/hooks"
  ensure_hook_dir "$RADIANT_HOOK_DIR"

  for hook_name in post-merge post-checkout pre-commit pre-push; do
    write_hook "$RADIANT_HOOK_DIR" "$hook_name" "vendor/radiant must use local 'next'" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK:-0}" == "1" ]]; then
  exit 0
fi

hook_name="$(basename "$0")"
if [[ "$hook_name" == "post-checkout" && "${3:-0}" != "1" ]]; then
  exit 0
fi

branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
if [[ -z "$branch" ]]; then
  exit 0
fi

if [[ "$branch" == "HEAD" ]]; then
  echo "[branch_guard] ERROR: vendor/radiant must use local 'next'; detached HEAD is not allowed." >&2
  exit 1
fi

if [[ "$branch" != "next" ]]; then
  echo "[branch_guard] ERROR: vendor/radiant must use local 'next'. Current branch: '$branch'." >&2
  exit 1
fi

upstream="$(git for-each-ref --format='%(upstream:short)' "refs/heads/$branch")"
if [[ -z "$upstream" ]]; then
  echo "[branch_guard] ERROR: vendor/radiant branch '$branch' has no upstream. Expected 'origin/next'." >&2
  exit 1
fi

if [[ "$upstream" != "origin/next" ]]; then
  echo "[branch_guard] ERROR: vendor/radiant branch '$branch' must track 'origin/next'. Current upstream: '$upstream'." >&2
  exit 1
fi
EOF
  done
fi

echo "[agent_hook_install] Installed hooks:"
echo "[agent_hook_install]   - $ROOT_HOOK_DIR/post-merge"
echo "[agent_hook_install]   - $ROOT_HOOK_DIR/post-checkout"
echo "[agent_hook_install]   - $ROOT_HOOK_DIR/pre-commit"
echo "[agent_hook_install]   - $ROOT_HOOK_DIR/pre-push"
if [[ -n "$RADIANT_HOOK_DIR" ]]; then
  echo "[agent_hook_install]   - $RADIANT_HOOK_DIR/post-merge"
  echo "[agent_hook_install]   - $RADIANT_HOOK_DIR/post-checkout"
  echo "[agent_hook_install]   - $RADIANT_HOOK_DIR/pre-commit"
  echo "[agent_hook_install]   - $RADIANT_HOOK_DIR/pre-push"
fi
echo "[agent_hook_install] Override with: export SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1"
