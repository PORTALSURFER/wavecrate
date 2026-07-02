#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_DIR="dist/artifacts"
OUT_DIR="dist/release"
CHECKSUM_NAME=""

usage() {
  cat <<'USAGE'
Usage: assemble_release_files.sh --checksum-name <name> [--artifact-dir <path>] [--out-dir <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact-dir)
      ARTIFACT_DIR="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
      shift 2
      ;;
    --checksum-name)
      CHECKSUM_NAME="${2:-}"
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

if [[ -z "$CHECKSUM_NAME" ]]; then
  echo "Missing --checksum-name" >&2
  usage >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
shopt -s nullglob
zips=("${ARTIFACT_DIR}"/*.zip)
entries=("${ARTIFACT_DIR}"/checksums-entry-*.txt)
if [[ ${#zips[@]} -eq 0 ]]; then
  echo "No release zip files found in $ARTIFACT_DIR." >&2
  exit 1
fi
if [[ ${#entries[@]} -eq 0 ]]; then
  echo "No checksums entry files found in $ARTIFACT_DIR." >&2
  exit 1
fi

cp "${zips[@]}" "$OUT_DIR/"
cat "${entries[@]}" > "${OUT_DIR}/${CHECKSUM_NAME}"
