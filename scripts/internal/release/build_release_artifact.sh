#!/usr/bin/env bash
set -euo pipefail

TARGET=""
PLATFORM=""
ARCH=""
CHANNEL=""
VERSION=""
TARGET_VERSION=""
BUILD_NUMBER=""
GIT_SHA=""
BUILD_DATE=""
OUT_DIR="dist/release"

usage() {
  cat <<'USAGE'
Usage: build_release_artifact.sh --target <triple> --platform <label> --arch <label> --channel <nightly|rc|stable> --version <semver> --target-version <x.y.z> --build-number <n> --git-sha <sha> --build-date <YYYY-MM-DD> [--out-dir <path>]
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="${2:-}"
      shift 2
      ;;
    --platform)
      PLATFORM="${2:-}"
      shift 2
      ;;
    --arch)
      ARCH="${2:-}"
      shift 2
      ;;
    --channel)
      CHANNEL="${2:-}"
      shift 2
      ;;
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --target-version)
      TARGET_VERSION="${2:-}"
      shift 2
      ;;
    --build-number)
      BUILD_NUMBER="${2:-}"
      shift 2
      ;;
    --git-sha)
      GIT_SHA="${2:-}"
      shift 2
      ;;
    --build-date)
      BUILD_DATE="${2:-}"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="${2:-}"
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

for required in TARGET PLATFORM ARCH CHANNEL VERSION TARGET_VERSION BUILD_NUMBER GIT_SHA BUILD_DATE; do
  if [[ -z "${!required}" ]]; then
    echo "Missing required argument for $required" >&2
    usage >&2
    exit 1
  fi
done

scripts/internal/release/build_release_zip.sh \
  --target "$TARGET" \
  --platform "$PLATFORM" \
  --arch "$ARCH" \
  --channel "$CHANNEL" \
  --version "$VERSION" \
  --target-version "$TARGET_VERSION" \
  --build-number "$BUILD_NUMBER" \
  --git-sha "$GIT_SHA" \
  --build-date "$BUILD_DATE" \
  --out-dir "$OUT_DIR"

mv "${OUT_DIR}/checksums-entry.txt" \
  "${OUT_DIR}/checksums-entry-${PLATFORM}-${ARCH}.txt"
