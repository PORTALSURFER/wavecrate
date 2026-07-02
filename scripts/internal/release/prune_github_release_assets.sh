#!/usr/bin/env bash
set -euo pipefail

TAG_NAME=""
REPOSITORY=""
RELEASE_DIR="dist/release"

usage() {
  cat <<'USAGE'
Usage: prune_github_release_assets.sh --tag <tag> --repo <owner/name> [--release-dir <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG_NAME="${2:-}"
      shift 2
      ;;
    --repo)
      REPOSITORY="${2:-}"
      shift 2
      ;;
    --release-dir)
      RELEASE_DIR="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$TAG_NAME" || -z "$REPOSITORY" ]]; then
  echo "Missing --tag or --repo" >&2
  usage >&2
  exit 1
fi

current_assets="$(mktemp)"
remote_assets="$(mktemp)"
cleanup() {
  rm -f "$current_assets" "$remote_assets"
}
trap cleanup EXIT

for asset_path in "$RELEASE_DIR"/*; do
  [[ -f "$asset_path" ]] || continue
  basename "$asset_path"
done | sort > "$current_assets"

if ! gh release view "$TAG_NAME" --repo "$REPOSITORY" --json assets --jq '.assets[].name' >"$remote_assets" 2>/dev/null; then
  exit 0
fi

while IFS= read -r asset_name; do
  if [[ -z "$asset_name" ]]; then
    continue
  fi
  if ! grep -Fxq "$asset_name" "$current_assets"; then
    gh release delete-asset "$TAG_NAME" "$asset_name" --repo "$REPOSITORY" --yes
  fi
done <"$remote_assets"
