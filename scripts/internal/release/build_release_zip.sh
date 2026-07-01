#!/usr/bin/env bash
set -euo pipefail

APP_NAME="wavecrate"
APP_DISPLAY_NAME="Wavecrate"
APP_BUNDLE_ID="org.portalsurfer.wavecrate"
APP_ICON_SOURCE="assets/logo3.icns"
REPO_ROOT="$(pwd)"
BUILD_CARGO_BIN="${WAVECRATE_CARGO_BIN:-cargo}"
SKIP_BUILD="${WAVECRATE_SKIP_BUILD:-0}"
MACOS_SIGNING="${WAVECRATE_MACOS_SIGNING:-0}"
OUT_DIR="dist/release"
TARGET=""
PLATFORM=""
ARCH=""
CHANNEL=""
VERSION=""
TARGET_VERSION=""
BUILD_NUMBER=""
GIT_SHA=""
BUILD_DATE=""

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
Usage: build_release_zip.sh --target <triple> --platform <label> --arch <label> --channel <stable|rc|nightly> --version <semver> [--target-version <x.y.z>] [--build-number <n>] [--git-sha <sha>] [--build-date <YYYY-MM-DD>] [--out-dir <path>]
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
    --target-version)
      TARGET_VERSION="$2"
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
    --build-date)
      BUILD_DATE="$2"
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

if [[ -z "$TARGET" || -z "$PLATFORM" || -z "$ARCH" || -z "$CHANNEL" || -z "$VERSION" ]]; then
  usage >&2
  exit 1
fi

if [[ -z "$TARGET_VERSION" ]]; then
  TARGET_VERSION="$(awk -F '"' '/^version =/ { print $2; exit }' Cargo.toml)"
fi
if [[ -z "$TARGET_VERSION" || ! "$TARGET_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Target version must be MAJOR.MINOR.PATCH." >&2
  exit 1
fi
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-(rc|nightly)\.[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "Version must be stable, rc, or nightly semver." >&2
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
if [[ -z "$BUILD_DATE" ]]; then
  BUILD_DATE="$(date -u '+%Y-%m-%d')"
fi
if [[ ! "$BUILD_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "Build date must be YYYY-MM-DD." >&2
  exit 1
fi

case "$CHANNEL" in
  stable)
    if [[ "$VERSION" != "$TARGET_VERSION" ]]; then
      echo "Stable release version must equal target version." >&2
      exit 1
    fi
    ZIP_NAME="${APP_NAME}-${VERSION}-${PLATFORM}-${ARCH}.zip"
    ;;
  rc)
    if [[ ! "$VERSION" =~ ^${TARGET_VERSION//./\\.}-rc\.[0-9]+$ ]]; then
      echo "RC release version must be ${TARGET_VERSION}-rc.N." >&2
      exit 1
    fi
    ZIP_NAME="${APP_NAME}-${VERSION}-${PLATFORM}-${ARCH}.zip"
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
  env_args+=("WAVECRATE_RELEASE_VERSION=$VERSION")
  env_args+=("WAVECRATE_RELEASE_CHANNEL=$CHANNEL")
  env_args+=("WAVECRATE_RELEASE_TARGET_VERSION=$TARGET_VERSION")
  env_args+=("WAVECRATE_RELEASE_BUILD_DATE=$BUILD_DATE")
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

write_update_manifest() {
  python3 - "$ROOT_DIR" "$APP_NAME" "$CHANNEL" "$TARGET" "$PLATFORM" "$ARCH" "$VERSION" "$TARGET_VERSION" "$GIT_SHA" "$BUILD_DATE" <<'PY'
import json
import sys
from pathlib import Path

root, app, channel, target, platform, arch, version, target_version, commit, build_date = sys.argv[1:]
root_path = Path(root)
files = sorted(
    path.relative_to(root_path).as_posix()
    for path in root_path.rglob("*")
    if path.is_file()
)
files.append("update-manifest.json")
manifest = {
    "app": app,
    "channel": channel,
    "target": target,
    "platform": platform,
    "arch": arch,
    "version": version,
    "target_version": target_version,
    "commit": commit,
    "build_date": build_date,
    "files": files,
}
(root_path / "update-manifest.json").write_text(
    json.dumps(manifest, indent=2) + "\n",
    encoding="utf-8",
)
PY
}

create_macos_app_bundle() {
  local executable_source="$1"
  local app_bundle="${ROOT_DIR}/${APP_DISPLAY_NAME}.app"
  local contents_dir="${app_bundle}/Contents"
  local macos_dir="${contents_dir}/MacOS"
  local resources_dir="${contents_dir}/Resources"
  local icon_name
  local bundle_version
  local short_version

  icon_name="$(basename "$APP_ICON_SOURCE")"
  bundle_version="${BUILD_NUMBER:-0}"
  short_version="${TARGET_VERSION}"

  mkdir -p "$macos_dir" "$resources_dir"
  cp "$executable_source" "${macos_dir}/${APP_NAME}"
  chmod 755 "${macos_dir}/${APP_NAME}"
  cp "$APP_ICON_SOURCE" "${resources_dir}/${icon_name}"

  cat > "${contents_dir}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_DISPLAY_NAME}</string>
  <key>CFBundleExecutable</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIconFile</key>
  <string>${icon_name}</string>
  <key>CFBundleIdentifier</key>
  <string>${APP_BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${APP_DISPLAY_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${short_version}</string>
  <key>CFBundleVersion</key>
  <string>${bundle_version}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

  if is_truthy "$MACOS_SIGNING"; then
    scripts/internal/release/sign_notarize_macos_app.sh --app "$app_bundle"
  fi
}

create_zip() {
  if [[ "$PLATFORM" == "macos" ]] && command -v ditto >/dev/null 2>&1; then
    (cd "$WORK_DIR" && ditto -c -k --sequesterRsrc --keepParent "$APP_NAME" "$ZIP_PATH")
  elif command -v zip >/dev/null 2>&1; then
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
    echo "No zip tool found (zip, ditto, or powershell Compress-Archive required)." >&2
    exit 1
  fi
}

if [[ "$PLATFORM" == "macos" ]]; then
  create_macos_app_bundle "target/${TARGET}/release/${BIN_NAME}"
else
  cp "target/${TARGET}/release/${BIN_NAME}" "${ROOT_DIR}/${BIN_NAME}"
fi
write_update_manifest

mkdir -p "$OUT_DIR"
if [[ "$OUT_DIR" = /* ]]; then
  ZIP_PATH="${OUT_DIR}/${ZIP_NAME}"
else
  ZIP_PATH="${REPO_ROOT}/${OUT_DIR}/${ZIP_NAME}"
fi
create_zip

if command -v sha256sum >/dev/null 2>&1; then
  SHA=$(sha256sum "$ZIP_PATH" | awk '{print $1}')
else
  SHA=$(shasum -a 256 "$ZIP_PATH" | awk '{print $1}')
fi
printf "%s  %s\n" "$SHA" "$ZIP_NAME" > "${OUT_DIR}/checksums-entry.txt"
