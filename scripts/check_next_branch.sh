#!/usr/bin/env bash

# Verify that the current repository uses the shared development branch.
#
# The script fails unless the repo root is on local `next` and that branch
# tracks `origin/next`. Use it from hooks and validation scripts to keep local
# work on the agreed branch.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

EXPECTED_BRANCH="next"
EXPECTED_UPSTREAM="origin/next"

usage() {
  cat <<'USAGE'
Usage: scripts/check_next_branch.sh

Fail unless the current repository is on local next tracking origin/next.
USAGE
}

while (( $# > 0 )); do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[branch_guard] Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

branch="$(git -C "$ROOT_DIR" rev-parse --abbrev-ref HEAD)"
if [[ "$branch" == "HEAD" ]]; then
  echo "[branch_guard] Detached HEAD is not allowed. Switch this repository to local '$EXPECTED_BRANCH'." >&2
  exit 1
fi

if [[ "$branch" != "$EXPECTED_BRANCH" ]]; then
  echo "[branch_guard] Development must happen on '$EXPECTED_BRANCH'. Current branch: '$branch'." >&2
  exit 1
fi

upstream="$(git -C "$ROOT_DIR" for-each-ref --format='%(upstream:short)' "refs/heads/$branch")"
if [[ -z "$upstream" ]]; then
  echo "[branch_guard] Branch '$branch' has no upstream. Set it to '$EXPECTED_UPSTREAM'." >&2
  exit 1
fi

if [[ "$upstream" != "$EXPECTED_UPSTREAM" ]]; then
  echo "[branch_guard] Branch '$branch' must track '$EXPECTED_UPSTREAM'. Current upstream: '$upstream'." >&2
  exit 1
fi

echo "[branch_guard] OK ($branch -> $upstream)"
