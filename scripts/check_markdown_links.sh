#!/usr/bin/env bash

# Diff-aware local Markdown link checker.
#
# Checks only added/modified Markdown files (plus staged/unstaged edits) for
# broken local file links so new link rot doesn't get introduced.
#
# What is checked:
# - Relative links to local files (e.g. `docs/FOO.md`, `../manual/usage.md`)
#
# What is ignored:
# - HTTP(S), mailto, tel
# - Absolute site links starting with `/`
# - Pure anchors starting with `#`

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ -f "$ROOT_DIR/scripts/git_diff_env.sh" ]]; then
  # shellcheck source=scripts/git_diff_env.sh
  source "$ROOT_DIR/scripts/git_diff_env.sh"
else
  sempal_git() {
    git "$@"
  }
fi

BASE_REF=""
HEAD_REF="HEAD"

usage() {
  cat <<'EOF'
Usage: scripts/check_markdown_links.sh [--base <ref>] [--head <ref>]

Checks changed Markdown files for broken local file links.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --base)
      BASE_REF="${2:-}"; shift 2 ;;
    --head)
      HEAD_REF="${2:-}"; shift 2 ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "[md_links] Unknown argument: $1" >&2
      usage >&2
      exit 2 ;;
  esac
done

git_has_commit() {
  sempal_git rev-parse --verify --quiet "$1^{commit}" >/dev/null 2>&1
}

collect_markdown_files() {
  local base="$1"
  local head="$2"

  local out=()
  if [[ -n "$base" ]] && git_has_commit "$base" && git_has_commit "$head"; then
    mapfile -t out < <(sempal_git diff --name-only --diff-filter=AM "$base...$head" -- '*.md' || true)
  elif git_has_commit "$head"; then
    mapfile -t out < <(sempal_git show --name-only --pretty=format: "$head" -- '*.md' || true)
  fi

  mapfile -t staged < <(sempal_git diff --name-only --diff-filter=AM --cached -- '*.md' || true)
  mapfile -t unstaged < <(sempal_git diff --name-only --diff-filter=AM -- '*.md' || true)

  printf "%s\n" "${out[@]}" "${staged[@]}" "${unstaged[@]}" \
    | sed 's#^\\./##' \
    | sort -u || true
}

files="$(collect_markdown_files "$BASE_REF" "$HEAD_REF")"
if [[ -z "${files:-}" ]]; then
  echo "[md_links] No changed Markdown files detected."
  exit 0
fi

printf "%s\n" "$files" | python3 - <<'PY'
import os
import re
import sys
from pathlib import Path

ROOT = Path.cwd()

def is_ignored(dest: str) -> bool:
    dest = dest.strip()
    if not dest:
        return True
    lower = dest.lower()
    if lower.startswith(("http://", "https://", "mailto:", "tel:")):
        return True
    if dest.startswith("#"):
        return True
    if dest.startswith("/"):
        return True
    return False

def strip_anchor_and_query(dest: str) -> str:
    # Keep it simple: split off query/fragment.
    dest = dest.split("#", 1)[0]
    dest = dest.split("?", 1)[0]
    return dest.strip()

link_re = re.compile(r"!?\\[[^\\]]*\\]\\(([^)]+)\\)")

violations = []

for line in sys.stdin:
    file_path = line.strip()
    if not file_path:
        continue
    path = ROOT / file_path
    if not path.is_file():
        continue
    text = path.read_text(encoding="utf-8", errors="replace")
    for match in link_re.finditer(text):
        dest_raw = match.group(1).strip()
        if is_ignored(dest_raw):
            continue
        dest = strip_anchor_and_query(dest_raw)
        if not dest or is_ignored(dest):
            continue
        # Ignore templated or variable links.
        if "${" in dest or "{{" in dest:
            continue
        resolved = (path.parent / dest).resolve()
        try:
            # Ensure we don't escape repo root inadvertently; still allow links outside docs
            # if they exist (repo-relative paths like `../README.md`).
            pass
        except Exception:
            pass
        if not resolved.exists():
            violations.append((file_path, dest_raw))

if violations:
    print("[md_links] Broken local file links detected:", file=sys.stderr)
    for file_path, dest in violations:
        print(f" - {file_path}: ({dest})", file=sys.stderr)
    sys.exit(1)

print("[md_links] OK")
PY
