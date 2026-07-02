#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${RADIANT_SUBMODULE_DEPLOY_KEY:-}" ]]; then
  echo "Missing RADIANT_SUBMODULE_DEPLOY_KEY secret." >&2
  exit 1
fi

mkdir -p ~/.ssh
key_path="$HOME/.ssh/radiant_submodule_key"
cleanup() {
  rm -f "$key_path"
}
trap cleanup EXIT

printf '%s\n' "$RADIANT_SUBMODULE_DEPLOY_KEY" > "$key_path"
chmod 600 "$key_path"
ssh-keyscan github.com >> ~/.ssh/known_hosts
git config submodule.vendor/radiant.url git@github.com:PORTALSURFER/radiant.git
GIT_SSH_COMMAND="ssh -i $key_path -o IdentitiesOnly=yes" \
  git submodule update --init --recursive vendor/radiant
