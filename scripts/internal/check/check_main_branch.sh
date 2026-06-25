#!/usr/bin/env bash

# Verify that the current repository uses main as its integration branch.
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
Usage: scripts/internal/check/check_main_branch.sh

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

integration_upstream="$(git -C "$ROOT_DIR" for-each-ref --format='%(upstream:short)' "refs/heads/$EXPECTED_BRANCH")"
if [[ -z "$integration_upstream" ]]; then
  echo "[branch_guard] Local '$EXPECTED_BRANCH' must exist and track '$EXPECTED_UPSTREAM'." >&2
  exit 1
fi

if [[ "$integration_upstream" != "$EXPECTED_UPSTREAM" ]]; then
  echo "[branch_guard] Local '$EXPECTED_BRANCH' must track '$EXPECTED_UPSTREAM'. Current upstream: '$integration_upstream'." >&2
  exit 1
fi

if [[ "$branch" == "$EXPECTED_BRANCH" ]]; then
  echo "[branch_guard] OK ($branch -> $integration_upstream)"
else
  echo "[branch_guard] OK (feature branch '$branch', base $EXPECTED_BRANCH -> $integration_upstream)"
fi
