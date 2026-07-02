#!/usr/bin/env bash
set -euo pipefail

TITLE="Wavecrate release summary"
STATUS="unknown"
CHANNEL=""
VERSION=""
TARGET_VERSION=""
COMMIT=""
BUILD_ID=""
BUILD_NUMBER=""
GITHUB_RELEASE_URL=""
GITHUB_TAG=""
PORTAL_CATALOG_URL=""
PORTAL_BUILD_ID=""
ARTIFACT_DIR=""
CHECKSUM_FILE=""
NOTES=()

usage() {
  cat <<'USAGE'
Usage: write_release_step_summary.sh [options]

Appends a secret-safe Markdown release summary to GITHUB_STEP_SUMMARY when that
environment variable is set. All values must be public release metadata.

Options:
  --title <text>
  --status <text>
  --channel <nightly|rc|stable>
  --version <version>
  --target-version <version>
  --commit <sha>
  --build-id <id>
  --build-number <number>
  --github-release-url <url>
  --github-tag <tag>
  --portal-catalog-url <url>
  --portal-build-id <id>
  --artifact-dir <path>
  --checksum-file <path>
  --note <text>
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --title)
      TITLE="${2:-}"
      shift 2
      ;;
    --status)
      STATUS="${2:-}"
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
    --commit)
      COMMIT="${2:-}"
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
    --github-release-url)
      GITHUB_RELEASE_URL="${2:-}"
      shift 2
      ;;
    --github-tag)
      GITHUB_TAG="${2:-}"
      shift 2
      ;;
    --portal-catalog-url)
      PORTAL_CATALOG_URL="${2:-}"
      shift 2
      ;;
    --portal-build-id)
      PORTAL_BUILD_ID="${2:-}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIR="${2:-}"
      shift 2
      ;;
    --checksum-file)
      CHECKSUM_FILE="${2:-}"
      shift 2
      ;;
    --note)
      NOTES+=("${2:-}")
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

summary_file="${GITHUB_STEP_SUMMARY:-}"
if [[ -z "$summary_file" ]]; then
  exit 0
fi

cell() {
  local value="${1:-}"
  value="${value//$'\n'/ }"
  value="${value//$'\r'/ }"
  value="${value//|/\\|}"
  if [[ -z "$value" ]]; then
    value="not available"
  fi
  printf "%s" "$value"
}

row() {
  local label="$1"
  local value="$2"
  printf '| %s | %s |\n' "$(cell "$label")" "$(cell "$value")"
}

artifact_rows=()
if [[ -n "$ARTIFACT_DIR" && -d "$ARTIFACT_DIR" ]]; then
  while IFS= read -r -d '' artifact; do
    name="$(basename "$artifact")"
    size_bytes="$(wc -c < "$artifact" | tr -d '[:space:]')"
    sha256="$(sha256sum "$artifact" | awk '{ print $1 }')"
    artifact_rows+=("| $(cell "$name") | $(cell "$size_bytes") | $(cell "$sha256") |")
  done < <(find "$ARTIFACT_DIR" -maxdepth 1 -type f -print0 | sort -z)
fi

checksum_name=""
if [[ -n "$CHECKSUM_FILE" ]]; then
  checksum_name="$(basename "$CHECKSUM_FILE")"
fi

{
  echo "## $(cell "$TITLE")"
  echo
  echo "| Field | Value |"
  echo "| --- | --- |"
  row "Status" "$STATUS"
  row "Channel" "$CHANNEL"
  row "Version" "$VERSION"
  row "Target version" "$TARGET_VERSION"
  row "Commit" "$COMMIT"
  row "Build ID" "$BUILD_ID"
  row "Build number" "$BUILD_NUMBER"
  row "GitHub tag" "$GITHUB_TAG"
  row "GitHub release" "$GITHUB_RELEASE_URL"
  row "PortalSurfer build ID" "$PORTAL_BUILD_ID"
  row "PortalSurfer catalog" "$PORTAL_CATALOG_URL"
  row "Checksum file" "$checksum_name"
  echo
  if [[ ${#artifact_rows[@]} -gt 0 ]]; then
    echo "| Artifact | Bytes | SHA-256 |"
    echo "| --- | ---: | --- |"
    printf '%s\n' "${artifact_rows[@]}"
  else
    echo "_No local release artifacts were available to summarize._"
  fi
  if [[ ${#NOTES[@]} -gt 0 ]]; then
    echo
    echo "### Notes"
    for note in "${NOTES[@]}"; do
      echo "- $(cell "$note")"
    done
  fi
  echo
} >> "$summary_file"
