#!/usr/bin/env bash

# Verify that the current repository uses main as its base branch.
#
# Feature branches are allowed for PR work, but local `main` must exist and
# track `origin/main`.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

EXPECTED_BRANCH="main"
EXPECTED_UPSTREAM="origin/main"

usage() {
  cat <<'USAGE'
Usage: scripts/internal/check/check_next_branch.sh

Verify that local main tracks origin/main; feature branches are allowed for PR work.
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
  echo "[branch_guard] Detached HEAD is not allowed. Use local '$EXPECTED_BRANCH' or a feature branch." >&2
  exit 1
fi

if [[ "$branch" == "next" ]]; then
  echo "[branch_guard] Local 'next' is retired. Use '$EXPECTED_BRANCH' as the base branch and feature branches for PR work." >&2
  exit 1
fi

main_upstream="$(git -C "$ROOT_DIR" for-each-ref --format='%(upstream:short)' "refs/heads/$EXPECTED_BRANCH")"
if [[ -z "$main_upstream" ]]; then
  echo "[branch_guard] Local '$EXPECTED_BRANCH' must exist and track '$EXPECTED_UPSTREAM'." >&2
  exit 1
fi

if [[ "$main_upstream" != "$EXPECTED_UPSTREAM" ]]; then
  echo "[branch_guard] Local '$EXPECTED_BRANCH' must track '$EXPECTED_UPSTREAM'. Current upstream: '$main_upstream'." >&2
  exit 1
fi

if [[ "$branch" == "$EXPECTED_BRANCH" ]]; then
  echo "[branch_guard] OK ($branch -> $main_upstream)"
else
  echo "[branch_guard] OK (feature branch '$branch', base $EXPECTED_BRANCH -> $main_upstream)"
fi
