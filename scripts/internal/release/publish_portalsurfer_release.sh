#!/usr/bin/env bash
set -euo pipefail

ARTIFACT_DIR="dist/release"
RELEASE_LOG="dist/release/release-log.md"
FULL_CHANGELOG_OUT="dist/release/changelog.md"
CHANNEL=""
BUILD_ID=""
BUILD_NUMBER=""
RELEASE_VERSION=""
RELEASED_AT=""

usage() {
  cat <<'USAGE'
Usage: publish_portalsurfer_release.sh --channel <nightly|rc|stable> --build-id <id> --build-number <n> --release-version <version> --released-at <iso-time> [--artifact-dir <path>] [--release-log <path>] [--full-changelog-out <path>]

Requires PORTALSURFER_RELEASE_UPLOAD_TOKEN. Uses PORTALSURFER_RELEASE_UPLOAD_URL
when set, otherwise https://portalsurfer.org/wavecrate/api/v1/release-uploads.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel)
      CHANNEL="${2:-}"
      shift 2
      ;;
    --build-id)
      BUILD_ID="${2:-}"
      shift 2
      ;;
    --build-number)
      BUILD_NUMBER="${2:-}"
      shift 2
      ;;
    --release-version)
      RELEASE_VERSION="${2:-}"
      shift 2
      ;;
    --released-at)
      RELEASED_AT="${2:-}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIR="${2:-}"
      shift 2
      ;;
    --release-log)
      RELEASE_LOG="${2:-}"
      shift 2
      ;;
    --full-changelog-out)
      FULL_CHANGELOG_OUT="${2:-}"
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

if [[ -z "$CHANNEL" || -z "$BUILD_ID" || -z "$BUILD_NUMBER" || -z "$RELEASE_VERSION" || -z "$RELEASED_AT" ]]; then
  usage >&2
  exit 1
fi
case "$CHANNEL" in
  nightly|rc|stable) ;;
  *)
    echo "channel must be nightly, rc, or stable." >&2
    exit 1
    ;;
esac
if [[ -z "${PORTALSURFER_RELEASE_UPLOAD_TOKEN:-}" ]]; then
  echo "Missing required secret: PORTALSURFER_RELEASE_UPLOAD_TOKEN" >&2
  exit 1
fi
if [[ ! "$BUILD_NUMBER" =~ ^[0-9]+$ ]]; then
  echo "build-number must be numeric." >&2
  exit 1
fi
if [[ ! -s "$RELEASE_LOG" ]]; then
  echo "Release log is missing or empty: $RELEASE_LOG" >&2
  exit 1
fi

upload_base="${PORTALSURFER_RELEASE_UPLOAD_URL:-https://portalsurfer.org/wavecrate/api/v1/release-uploads}"
upload_base="${upload_base%/}"
if [[ "$upload_base" != */release-uploads ]]; then
  echo "PORTALSURFER_RELEASE_UPLOAD_URL must end with /release-uploads." >&2
  exit 1
fi

api_base="${upload_base%/release-uploads}"
catalog_url="$api_base/releases"
full_changelog_url="$api_base/changelog"
response_dir="$(mktemp -d)"
trap 'rm -rf "$response_dir"' EXIT

shopt -s nullglob
files=("${ARTIFACT_DIR}"/*.zip "${ARTIFACT_DIR}"/checksums-*.txt "${ARTIFACT_DIR}"/checksums-*.txt.sig)
if [[ ${#files[@]} -eq 0 ]]; then
  echo "No release artifacts found in $ARTIFACT_DIR." >&2
  exit 1
fi

uploaded_names=()
for file in "${files[@]}"; do
  file_name="$(basename "$file")"
  if [[ ! -s "$file" ]]; then
    echo "Release file is empty: $file" >&2
    exit 1
  fi
  content_type="application/octet-stream"
  if [[ "$file_name" == *.zip ]]; then
    content_type="application/zip"
  fi
  sha256="$(sha256sum "$file" | awk '{ print $1 }')"
  encoded_name="$(python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$file_name")"
  response_file="$response_dir/$file_name.json"
  curl --fail-with-body --retry 3 --retry-delay 2 \
    --request PUT \
    --header "Authorization: Bearer $PORTALSURFER_RELEASE_UPLOAD_TOKEN" \
    --header "Content-Type: $content_type" \
    --header "X-Wavecrate-Build-Number: ${BUILD_NUMBER}" \
    --header "X-Wavecrate-Release-Channel: $CHANNEL" \
    --header "X-Wavecrate-Release-Version: $RELEASE_VERSION" \
    --header "X-Wavecrate-Released-At: $RELEASED_AT" \
    --header "X-Wavecrate-Sha256: $sha256" \
    --data-binary "@$file" \
    "$upload_base/$BUILD_ID/files/$encoded_name" > "$response_file"
  python3 - "$response_file" "$BUILD_ID" "$BUILD_NUMBER" "$file_name" "$sha256" <<'PY'
import json
import sys

response_path, expected_build, expected_build_number, expected_name, expected_sha = sys.argv[1:]
response = json.loads(open(response_path, encoding="utf-8").read())
release = response.get("release") or {}
file = response.get("file") or {}
if release.get("build_id") != expected_build:
    raise SystemExit(f"Uploaded release id mismatch: {release.get('build_id')} != {expected_build}")
if release.get("build_number") != int(expected_build_number):
    raise SystemExit(f"Uploaded release build number mismatch: {release.get('build_number')} != {expected_build_number}")
if file.get("name") != expected_name:
    raise SystemExit(f"Uploaded file name mismatch: {file.get('name')} != {expected_name}")
if file.get("sha256") != expected_sha:
    raise SystemExit(f"Uploaded file sha mismatch: {file.get('sha256')} != {expected_sha}")
if not isinstance(file.get("size_bytes"), int) or file["size_bytes"] <= 0:
    raise SystemExit("Uploaded file size was not recorded")
PY
  uploaded_names+=("$file_name")
done

changelog_response_file="$response_dir/changelog-upload.json"
curl --fail-with-body --retry 3 --retry-delay 2 \
  --request PUT \
  --header "Authorization: Bearer $PORTALSURFER_RELEASE_UPLOAD_TOKEN" \
  --header "Content-Type: text/markdown; charset=utf-8" \
  --header "X-Wavecrate-Build-Number: ${BUILD_NUMBER}" \
  --header "X-Wavecrate-Release-Channel: $CHANNEL" \
  --header "X-Wavecrate-Release-Version: $RELEASE_VERSION" \
  --header "X-Wavecrate-Released-At: $RELEASED_AT" \
  --data-binary "@$RELEASE_LOG" \
  "$upload_base/$BUILD_ID/changelog" > "$changelog_response_file"
python3 - "$changelog_response_file" "$BUILD_ID" "$BUILD_NUMBER" <<'PY'
import json
import sys

response_path, expected_build, expected_build_number = sys.argv[1:]
response = json.loads(open(response_path, encoding="utf-8").read())
release = response.get("release") or {}
changelog = response.get("changelog") or {}
if release.get("build_id") != expected_build:
    raise SystemExit(f"Uploaded changelog release id mismatch: {release.get('build_id')} != {expected_build}")
if release.get("build_number") != int(expected_build_number):
    raise SystemExit(f"Uploaded changelog build number mismatch: {release.get('build_number')} != {expected_build_number}")
if changelog.get("format") != "markdown":
    raise SystemExit("Uploaded changelog was not recorded as markdown")
if not changelog.get("url"):
    raise SystemExit("Uploaded changelog did not expose a public URL")
PY

catalog_file="$response_dir/releases.json"
curl --fail-with-body --retry 3 --retry-delay 2 "$catalog_url" > "$catalog_file"
python3 - "$catalog_file" "$BUILD_ID" "$BUILD_NUMBER" "$RELEASE_VERSION" "$RELEASED_AT" "${uploaded_names[@]}" <<'PY'
import json
import sys

catalog_path = sys.argv[1]
expected_build = sys.argv[2]
expected_build_number = int(sys.argv[3])
expected_version = sys.argv[4]
expected_released_at = sys.argv[5]
expected_files = set(sys.argv[6:])
catalog = json.loads(open(catalog_path, encoding="utf-8").read())
releases = catalog.get("releases") or []
release = next((item for item in releases if item.get("build_id") == expected_build), None)
if release is None:
    raise SystemExit(f"Release catalog does not list {expected_build}")
if release.get("build_number") != expected_build_number:
    raise SystemExit(f"Release catalog build number mismatch: {release.get('build_number')} != {expected_build_number}")
if release.get("version") != expected_version:
    raise SystemExit(f"Release catalog version mismatch: {release.get('version')} != {expected_version}")
if release.get("released_at") != expected_released_at:
    raise SystemExit(f"Release catalog timestamp mismatch: {release.get('released_at')} != {expected_released_at}")
catalog_files = {item.get("name") for item in release.get("files") or []}
missing = sorted(expected_files - catalog_files)
if missing:
    raise SystemExit(f"Release catalog is missing files: {', '.join(missing)}")
changelog = release.get("changelog") or {}
if changelog.get("format") != "markdown" or not changelog.get("url"):
    raise SystemExit("Release catalog is missing the markdown changelog link")
print(f"Uploaded {len(expected_files)} Wavecrate release file(s) to {expected_build}")
PY

changelog_catalog_file="$response_dir/changelog-catalog.json"
curl --fail-with-body --retry 3 --retry-delay 2 \
  "$catalog_url/$BUILD_ID/changelog" > "$changelog_catalog_file"
python3 - "$changelog_catalog_file" "$BUILD_ID" "$RELEASE_LOG" <<'PY'
import json
import sys

catalog_path, expected_build, changelog_path = sys.argv[1:]
response = json.loads(open(catalog_path, encoding="utf-8").read())
if response.get("build_id") != expected_build:
    raise SystemExit(f"Fetched changelog id mismatch: {response.get('build_id')} != {expected_build}")
body = (response.get("changelog") or {}).get("body", "").strip()
expected_body = open(changelog_path, encoding="utf-8").read().strip()
if body != expected_body:
    raise SystemExit("Fetched changelog body does not match generated release log")
print(f"Uploaded Wavecrate release log to {expected_build}")
PY

scripts/internal/release/assemble_portal_changelog.py \
  --catalog-url "$catalog_url" \
  --current-build-id "$BUILD_ID" \
  --current-log "$RELEASE_LOG" \
  --existing-changelog-url "$full_changelog_url" \
  --generated-at "$RELEASED_AT" \
  --output "$FULL_CHANGELOG_OUT"

full_changelog_response_file="$response_dir/full-changelog-upload.json"
curl --fail-with-body --retry 3 --retry-delay 2 \
  --request PUT \
  --header "Authorization: Bearer $PORTALSURFER_RELEASE_UPLOAD_TOKEN" \
  --header "Content-Type: text/markdown; charset=utf-8" \
  --header "X-Wavecrate-Changelog-Title: Wavecrate changelog" \
  --data-binary "@$FULL_CHANGELOG_OUT" \
  "$upload_base/changelog" > "$full_changelog_response_file"
python3 - "$full_changelog_response_file" <<'PY'
import json
import sys

response = json.loads(open(sys.argv[1], encoding="utf-8").read())
changelog = response.get("changelog") or {}
if changelog.get("format") != "markdown":
    raise SystemExit("Uploaded full changelog was not recorded as markdown")
if not changelog.get("url"):
    raise SystemExit("Uploaded full changelog did not expose a public URL")
PY

full_changelog_catalog_file="$response_dir/full-changelog-catalog.json"
curl --fail-with-body --retry 3 --retry-delay 2 \
  "$full_changelog_url" > "$full_changelog_catalog_file"
python3 - "$full_changelog_catalog_file" "$FULL_CHANGELOG_OUT" <<'PY'
import json
import sys

catalog_path, changelog_path = sys.argv[1:]
response = json.loads(open(catalog_path, encoding="utf-8").read())
body = (response.get("changelog") or {}).get("body", "").strip()
expected_body = open(changelog_path, encoding="utf-8").read().strip()
if body != expected_body:
    raise SystemExit("Fetched full changelog body does not match generated changelog")
print("Uploaded Wavecrate full changelog")
PY
