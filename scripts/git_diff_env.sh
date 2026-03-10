#!/usr/bin/env bash

# Shared git wrapper for diff-aware shell checks.
#
# Git Bash/WSL against `/mnt/<drive>/...` worktrees can misread CRLF-normalized
# Windows repos as fully dirty unless `core.autocrlf=true` is forced for the
# shell-side git invocations. This helper keeps diff-aware guardrails aligned
# with the Windows-native git view without changing repo config.

set -euo pipefail

sempal_git_uses_windows_worktree() {
  local current_dir
  current_dir="$(pwd -P 2>/dev/null || pwd)"
  [[ "$current_dir" =~ ^/mnt/[A-Za-z]/ ]]
}

sempal_git() {
  if sempal_git_uses_windows_worktree; then
    git -c core.autocrlf=true "$@"
  else
    git "$@"
  fi
}
