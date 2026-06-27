#!/usr/bin/env bash
set -euo pipefail

APP_PATH=""

usage() {
  cat <<'EOF'
Usage: sign_notarize_macos_app.sh --app <Wavecrate.app>

Required environment:
  APPLE_DEVELOPER_ID_APPLICATION_CERT_BASE64
  APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD
  APPLE_NOTARY_KEY_BASE64
  APPLE_NOTARY_KEY_ID
  APPLE_NOTARY_ISSUER_ID

Optional environment:
  APPLE_CODESIGN_IDENTITY
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app)
      APP_PATH="$2"
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

if [[ -z "$APP_PATH" || ! -d "$APP_PATH" ]]; then
  echo "A valid --app bundle path is required." >&2
  exit 1
fi
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS signing and notarization must run on macOS." >&2
  exit 1
fi

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required Apple release signing secret: $name" >&2
    exit 1
  fi
}

decode_base64_to_file() {
  local value="$1"
  local output="$2"
  if printf '%s' "$value" | base64 --decode > "$output" 2>/dev/null; then
    return 0
  fi
  printf '%s' "$value" | base64 -D > "$output"
}

require_env APPLE_DEVELOPER_ID_APPLICATION_CERT_BASE64
require_env APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD
require_env APPLE_NOTARY_KEY_BASE64
require_env APPLE_NOTARY_KEY_ID
require_env APPLE_NOTARY_ISSUER_ID

WORK_DIR="$(mktemp -d)"
KEYCHAIN_PATH="$WORK_DIR/wavecrate-release-signing.keychain-db"
KEYCHAIN_PASSWORD="$(uuidgen | tr -d '-')"
CERT_PATH="$WORK_DIR/developer-id-application.p12"
NOTARY_KEY_PATH="$WORK_DIR/AuthKey_${APPLE_NOTARY_KEY_ID}.p8"
NOTARY_ZIP_PATH="$WORK_DIR/notary-upload.zip"
ORIGINAL_KEYCHAINS_FILE="$WORK_DIR/original-keychains.txt"

cleanup() {
  if [[ -f "$ORIGINAL_KEYCHAINS_FILE" ]]; then
    original_keychains=()
    while IFS= read -r keychain; do
      [[ -n "$keychain" ]] && original_keychains+=("$keychain")
    done < "$ORIGINAL_KEYCHAINS_FILE"
    if [[ "${#original_keychains[@]}" -gt 0 ]]; then
      security list-keychains -d user -s "${original_keychains[@]}" >/dev/null 2>&1 || true
    fi
  fi
  security delete-keychain "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

decode_base64_to_file "$APPLE_DEVELOPER_ID_APPLICATION_CERT_BASE64" "$CERT_PATH"
decode_base64_to_file "$APPLE_NOTARY_KEY_BASE64" "$NOTARY_KEY_PATH"
chmod 600 "$CERT_PATH" "$NOTARY_KEY_PATH"

security list-keychains -d user | sed 's/[[:space:]]*"//g; s/"$//' > "$ORIGINAL_KEYCHAINS_FILE"
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
original_keychains=()
while IFS= read -r keychain; do
  [[ -n "$keychain" ]] && original_keychains+=("$keychain")
done < "$ORIGINAL_KEYCHAINS_FILE"
security list-keychains -d user -s "$KEYCHAIN_PATH" "${original_keychains[@]}"
security import "$CERT_PATH" \
  -P "$APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD" \
  -A \
  -t cert \
  -f pkcs12 \
  -k "$KEYCHAIN_PATH"
security set-key-partition-list \
  -S apple-tool:,apple:,codesign: \
  -s \
  -k "$KEYCHAIN_PASSWORD" \
  "$KEYCHAIN_PATH" >/dev/null

if [[ -n "${APPLE_CODESIGN_IDENTITY:-}" ]]; then
  CODESIGN_IDENTITY="$APPLE_CODESIGN_IDENTITY"
else
  CODESIGN_IDENTITY="$(
    security find-identity -v -p codesigning "$KEYCHAIN_PATH" |
      sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p' |
      head -n 1
  )"
fi

if [[ -z "$CODESIGN_IDENTITY" ]]; then
  echo "No Developer ID Application signing identity was found in the imported certificate." >&2
  exit 1
fi

find "$APP_PATH/Contents/MacOS" -type f -print0 |
  while IFS= read -r -d '' executable; do
    codesign \
      --force \
      --timestamp \
      --options runtime \
      --sign "$CODESIGN_IDENTITY" \
      "$executable"
  done

codesign \
  --force \
  --timestamp \
  --options runtime \
  --sign "$CODESIGN_IDENTITY" \
  "$APP_PATH"

codesign --verify --strict --verbose=4 "$APP_PATH"

ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$NOTARY_ZIP_PATH"
xcrun notarytool submit "$NOTARY_ZIP_PATH" \
  --key "$NOTARY_KEY_PATH" \
  --key-id "$APPLE_NOTARY_KEY_ID" \
  --issuer "$APPLE_NOTARY_ISSUER_ID" \
  --wait
xcrun stapler staple "$APP_PATH"
xcrun stapler validate "$APP_PATH"
spctl --assess --type execute --verbose=4 "$APP_PATH"
