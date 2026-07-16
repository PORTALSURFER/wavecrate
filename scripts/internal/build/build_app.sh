#!/usr/bin/env bash

# Build Wavecrate and, on macOS, wrap the exact binary in a uniquely identified
# app bundle that Launch Services and accessibility automation can address.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PROFILE="release"
VARIANT=""
SKIP_BUILD=0
CARGO_ARGS=()

usage() {
  cat <<'EOF'
Usage: scripts/build.sh [--release|--debug] [--variant <name>] [--no-build] [-- <cargo args>]

Builds the Wavecrate binary. On macOS, the exact binary is also copied into a
branch-specific app bundle under target/app-bundles so Launch Services and UI
automation do not fall back to an installed Wavecrate build.

Options:
  --release         Build the optimized release profile (default).
  --debug           Build the debug profile.
  --variant <name>  Override the app suffix, for example OPT-1179.
  --no-build        Re-bundle the existing profile binary without invoking Cargo.
  --                 Pass the remaining arguments to cargo build.
EOF
}

while (( $# > 0 )); do
  case "$1" in
    --release) PROFILE="release" ;;
    --debug) PROFILE="debug" ;;
    --variant)
      shift
      (( $# > 0 )) || { echo "[build][error] --variant requires a value" >&2; exit 2; }
      VARIANT="$1"
      ;;
    --no-build) SKIP_BUILD=1 ;;
    --help|-h) usage; exit 0 ;;
    --)
      shift
      CARGO_ARGS+=("$@")
      break
      ;;
    *)
      echo "[build][error] unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

cd "$ROOT_DIR"

if (( SKIP_BUILD == 0 )); then
  cargo_command=(cargo build --bin wavecrate)
  if [[ "$PROFILE" == "release" ]]; then
    cargo_command+=(--release)
  fi
  if (( ${#CARGO_ARGS[@]} > 0 )); then
    cargo_command+=("${CARGO_ARGS[@]}")
  fi
  printf '[build]'
  printf ' %q' "${cargo_command[@]}"
  printf '\n'
  "${cargo_command[@]}"
fi

profile_dir="$PROFILE"
binary_path="${CARGO_TARGET_DIR:-$ROOT_DIR/target}/${profile_dir}/wavecrate"
[[ -x "$binary_path" ]] || {
  echo "[build][error] Wavecrate binary not found at $binary_path" >&2
  exit 1
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "[build] binary=$binary_path"
  exit 0
fi

if [[ -z "$VARIANT" ]]; then
  branch="$(git branch --show-current 2>/dev/null || true)"
  if [[ "$branch" =~ ([Oo][Pp][Tt]-[0-9]+) ]]; then
    VARIANT="$(printf '%s' "${BASH_REMATCH[1]}" | tr '[:lower:]' '[:upper:]')"
  elif [[ -n "$branch" ]]; then
    VARIANT="$(printf '%s' "$branch" | tr -cs '[:alnum:]' '-' | sed 's/^-//; s/-$//' | cut -c1-40)"
  else
    VARIANT="detached-$(git rev-parse --short HEAD)"
  fi
fi

VARIANT="$(printf '%s' "$VARIANT" | tr -cs '[:alnum:]' '-' | sed 's/^-//; s/-$//' | cut -c1-40)"
[[ -n "$VARIANT" ]] || {
  echo "[build][error] app variant must contain at least one letter or number" >&2
  exit 2
}

identifier_suffix="$(printf '%s' "$VARIANT" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]')"
display_name="Wavecrate $VARIANT"
bundle_identifier="org.portalsurfer.wavecrate.dev.$identifier_suffix"
bundle_root="$ROOT_DIR/target/app-bundles/$identifier_suffix"
bundle_path="$bundle_root/$display_name.app"
contents_path="$bundle_path/Contents"
executable_path="$contents_path/MacOS/wavecrate"
info_path="$contents_path/Info.plist"

mkdir -p "$contents_path/MacOS"
install -m 755 "$binary_path" "$executable_path"

package_version="$(awk '
  /^\[package\][[:space:]]*$/ { in_package = 1; next }
  in_package && /^\[/ { exit }
  in_package && /^[[:space:]]*version[[:space:]]*=/ {
    gsub(/"/, "", $3)
    print $3
    exit
  }
' Cargo.toml)"
bundle_version="$(git rev-list --count HEAD)"

cat > "$info_path" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>$display_name</string>
  <key>CFBundleExecutable</key>
  <string>wavecrate</string>
  <key>CFBundleIdentifier</key>
  <string>$bundle_identifier</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$display_name</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$package_version</string>
  <key>CFBundleVersion</key>
  <string>$bundle_version</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

plutil -lint "$info_path" >/dev/null
codesign --force --deep --sign - "$bundle_path" >/dev/null
codesign --verify --deep --strict "$bundle_path"

echo "[build] binary=$binary_path"
echo "[build] app=$bundle_path"
echo "[build] bundle_id=$bundle_identifier"
echo "[build] computer_use_app=$display_name"
echo "[build] launch: open -na '$bundle_path' --args --log"
