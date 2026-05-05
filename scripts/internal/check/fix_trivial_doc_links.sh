#!/usr/bin/env bash

# Applies safe, mechanical doc link rewrites for known moved developer docs.
#
# This is intentionally conservative:
# - Only rewrites when the destination path exists.
# - Only touches tracked Markdown files in a small set of top-level locations.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

if ! command -v rg >/dev/null 2>&1; then
  echo "[fix_trivial_doc_links] ripgrep (rg) is required" >&2
  exit 2
fi

rewrite_in_file() {
  local file="$1"
  local from="$2"
  local to="$3"

  python3 - "$file" "$from" "$to" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
src = sys.argv[2]
dst = sys.argv[3]

text = path.read_text(encoding="utf-8")
new = text.replace(src, dst)
if new != text:
    path.write_text(new, encoding="utf-8")
PY
}

declare -a REWRITES=(
  "manual/gui_migration_parity.md" "docs/ARCHITECTURE.md"
  "manual/native_shell_legacy_baseline.md" "docs/SYSTEMS.md"
  "manual/performance_qa.md" "docs/SYSTEMS.md"
  "manual/feature_vector.md" "docs/SYSTEMS.md"
  "manual/ann_index_container.md" "docs/SYSTEMS.md"
  "manual/updater-contract.md" "docs/SYSTEMS.md"
  "manual/styleguide.md" "docs/ARCHITECTURE.md"
  "manual/icon_assets.md" "docs/SYSTEMS.md"
  "manual/hints.md" "docs/SYSTEMS.md"
  "manual/plan.md" "AGENTS.md"
  "manual/todo.md" "AGENTS.md"
  "manual/transient_plan.md" "AGENTS.md"
  "manual/transient_audit.md" "AGENTS.md"
  "manual/drag_audit.md" "AGENTS.md"
)

changed=0

for ((i=0; i<${#REWRITES[@]}; i+=2)); do
  from="${REWRITES[i]}"
  to="${REWRITES[i+1]}"
  if [[ ! -f "$to" ]]; then
    continue
  fi

  files=()
  while IFS= read -r path; do
    files+=("$path")
  done < <(rg -l --fixed-strings "$from" docs 2>/dev/null || true)
  if (( ${#files[@]} == 0 )); then
    continue
  fi

  for file in "${files[@]}"; do
    rewrite_in_file "$file" "$from" "$to"
    changed=1
  done
done

if (( changed == 0 )); then
  echo "[fix_trivial_doc_links] no changes" >&2
else
  echo "[fix_trivial_doc_links] rewrites applied" >&2
fi
