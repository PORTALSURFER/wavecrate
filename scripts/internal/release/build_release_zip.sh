#!/usr/bin/env bash
set -euo pipefail

APP_NAME="wavecrate"
REPO_ROOT="$(pwd)"
BUILD_CARGO_BIN="${WAVECRATE_CARGO_BIN:-cargo}"
SKIP_BUILD="${WAVECRATE_SKIP_BUILD:-0}"
OUT_DIR="dist/release"
TARGET=""
PLATFORM=""
ARCH=""
CHANNEL=""
VERSION=""
BUILD_NUMBER=""
GIT_SHA=""

is_truthy() {
  local value
  value="$(printf '%s' "${1:-}" | tr '[:upper:]' '[:lower:]')"
  case "$value" in
    1|true|yes|on) return 0 ;;
    *) return 1 ;;
  esac
}

usage() {
  cat <<'EOF'
Usage: build_release_zip.sh --target <triple> --platform <label> --arch <label> --channel <stable|nightly> [--version <x.y.z>] [--build-number <n>] [--git-sha <sha>] [--out-dir <path>]
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --platform)
      PLATFORM="$2"
      shift 2
      ;;
    --arch)
      ARCH="$2"
      shift 2
      ;;
    --channel)
      CHANNEL="$2"
      shift 2
      ;;
    --version)
      VERSION="$2"
      shift 2
      ;;
    --build-number)
      BUILD_NUMBER="$2"
      shift 2
      ;;
    --git-sha)
      GIT_SHA="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    -h|--help)
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

if [[ -z "$TARGET" || -z "$PLATFORM" || -z "$ARCH" || -z "$CHANNEL" ]]; then
  usage >&2
  exit 1
fi

BUILD_LABEL=""
if [[ -n "$BUILD_NUMBER" ]]; then
  BUILD_NUMBER="${BUILD_NUMBER#b}"
  if [[ -z "$BUILD_NUMBER" || ! "$BUILD_NUMBER" =~ ^[0-9]+$ ]]; then
    echo "Build number must be numeric." >&2
    exit 1
  fi
  BUILD_LABEL="-b${BUILD_NUMBER}"
fi
if [[ -n "$GIT_SHA" ]]; then
  GIT_SHA="$(printf '%s' "$GIT_SHA" | tr -d '[:space:]')"
  if [[ -z "$GIT_SHA" || ! "$GIT_SHA" =~ ^[0-9a-fA-F]{7,40}$ ]]; then
    echo "Git SHA must be a 7-40 character hexadecimal SHA." >&2
    exit 1
  fi
fi

case "$CHANNEL" in
  stable)
    if [[ -z "$VERSION" ]]; then
      echo "Stable releases require --version." >&2
      exit 1
    fi
    ZIP_NAME="${APP_NAME}-v${VERSION}${BUILD_LABEL}-${PLATFORM}-${ARCH}.zip"
    ;;
  nightly)
    ZIP_NAME="${APP_NAME}-nightly${BUILD_LABEL}-${PLATFORM}-${ARCH}.zip"
    ;;
  *)
    echo "Unknown channel: $CHANNEL" >&2
    exit 1
    ;;
esac

if ! is_truthy "$SKIP_BUILD"; then
  env_args=()
  if [[ -n "$BUILD_NUMBER" ]]; then
    env_args+=("WAVECRATE_BUILD_NUMBER=$BUILD_NUMBER")
  fi
  if [[ -n "$GIT_SHA" ]]; then
    env_args+=("WAVECRATE_GIT_SHA=$GIT_SHA")
  fi
  env "${env_args[@]}" "$BUILD_CARGO_BIN" build --release -p "$APP_NAME" --bin "$APP_NAME" --target "$TARGET"
fi

BIN_NAME="$APP_NAME"
if [[ "$TARGET" == *windows* ]]; then
  BIN_NAME="${APP_NAME}.exe"
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

ROOT_DIR="${WORK_DIR}/${APP_NAME}"
mkdir -p "$ROOT_DIR"
cp "target/${TARGET}/release/${BIN_NAME}" "${ROOT_DIR}/${BIN_NAME}"

mkdir -p "$OUT_DIR"
ZIP_PATH="${REPO_ROOT}/${OUT_DIR}/${ZIP_NAME}"
if command -v zip >/dev/null 2>&1; then
  (cd "$WORK_DIR" && zip -r "$ZIP_PATH" "$APP_NAME" >/dev/null)
elif command -v powershell.exe >/dev/null 2>&1; then
  mkdir -p "$OUT_DIR"
  if command -v cygpath >/dev/null 2>&1; then
    POWERSHELL_OUT_DIR=$(cygpath -w "$OUT_DIR")
    POWERSHELL_WORK_DIR=$(cygpath -w "$WORK_DIR")
  else
    POWERSHELL_OUT_DIR=$(powershell.exe -NoProfile -Command "[System.IO.Path]::GetFullPath('$OUT_DIR')")
    POWERSHELL_WORK_DIR=$(powershell.exe -NoProfile -Command "[System.IO.Path]::GetFullPath('$WORK_DIR')")
  fi
  POWERSHELL_ZIP_PATH="$POWERSHELL_OUT_DIR\\${ZIP_NAME}"
  powershell.exe -NoProfile -Command "Compress-Archive -Path \"$POWERSHELL_WORK_DIR\\$APP_NAME\\*\" -DestinationPath \"$POWERSHELL_ZIP_PATH\" -Force"
else
  echo "No zip tool found (zip or powershell Compress-Archive required)." >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  SHA=$(sha256sum "${OUT_DIR}/${ZIP_NAME}" | awk '{print $1}')
else
  SHA=$(shasum -a 256 "${OUT_DIR}/${ZIP_NAME}" | awk '{print $1}')
fi
printf "%s  %s\n" "$SHA" "$ZIP_NAME" > "${OUT_DIR}/checksums-entry.txt"
