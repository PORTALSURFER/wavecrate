#!/usr/bin/env bash
set -euo pipefail

CHANNEL=""
VERSION=""
TARGET_VERSION=""
TARGET_SHA=""
TARGET_BRANCH=""
BUILD_DATE=""
ARTIFACT_DIR=""
CHECKSUM_NAME=""
CHECKSUM_SIG_NAME=""
OUT_FILE=""
RC_NUMBER=""
PROMOTED_RC_TAG=""
RELEASE_TAG=""
CLIFF_CONFIG="cliff.toml"

usage() {
  cat <<'USAGE'
Usage: generate_release_log.sh --channel <rc|stable> --version <version> --target-sha <sha> --target-branch <branch> --build-date <YYYY-MM-DD> --artifact-dir <path> --checksum-name <name> --checksum-sig-name <name> --out <path> [options]

Options:
  --target-version <x.y.z>       Stable target version for RC logs.
  --rc-number <n>                RC number for RC logs.
  --promoted-rc-tag <tag>        Promoted RC tag for stable logs.
  --release-tag <tag>            Git tag for the release being generated.
  --cliff-config <path>          git-cliff config path. Defaults to cliff.toml.

Manual notes can be supplied with RELEASE_NOTES.
Set WAVECRATE_RELEASE_LOG_DISABLE_GIT_CLIFF=1 to force the deterministic git-log fallback.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
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
    --target-sha)
      TARGET_SHA="${2:-}"
      shift 2
      ;;
    --target-branch)
      TARGET_BRANCH="${2:-}"
      shift 2
      ;;
    --build-date)
      BUILD_DATE="${2:-}"
      shift 2
      ;;
    --artifact-dir)
      ARTIFACT_DIR="${2:-}"
      shift 2
      ;;
    --checksum-name)
      CHECKSUM_NAME="${2:-}"
      shift 2
      ;;
    --checksum-sig-name)
      CHECKSUM_SIG_NAME="${2:-}"
      shift 2
      ;;
    --out)
      OUT_FILE="${2:-}"
      shift 2
      ;;
    --rc-number)
      RC_NUMBER="${2:-}"
      shift 2
      ;;
    --promoted-rc-tag)
      PROMOTED_RC_TAG="${2:-}"
      shift 2
      ;;
    --release-tag)
      RELEASE_TAG="${2:-}"
      shift 2
      ;;
    --cliff-config)
      CLIFF_CONFIG="${2:-}"
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

require_arg() {
  local value="$1"
  local name="$2"
  if [[ -z "$value" ]]; then
    echo "Missing required argument: $name" >&2
    exit 1
  fi
}

require_arg "$CHANNEL" "--channel"
require_arg "$VERSION" "--version"
require_arg "$TARGET_SHA" "--target-sha"
require_arg "$TARGET_BRANCH" "--target-branch"
require_arg "$BUILD_DATE" "--build-date"
require_arg "$ARTIFACT_DIR" "--artifact-dir"
require_arg "$CHECKSUM_NAME" "--checksum-name"
require_arg "$CHECKSUM_SIG_NAME" "--checksum-sig-name"
require_arg "$OUT_FILE" "--out"

if [[ "$CHANNEL" != "rc" && "$CHANNEL" != "stable" ]]; then
  echo "channel must be rc or stable" >&2
  exit 1
fi
if [[ ! "$BUILD_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
  echo "build date must be YYYY-MM-DD" >&2
  exit 1
fi
if [[ "$CHANNEL" == "rc" ]]; then
  require_arg "$TARGET_VERSION" "--target-version"
  require_arg "$RC_NUMBER" "--rc-number"
  if [[ ! "$RC_NUMBER" =~ ^[1-9][0-9]*$ ]]; then
    echo "rc number must be a positive integer" >&2
    exit 1
  fi
fi
if [[ "$CHANNEL" == "stable" ]]; then
  require_arg "$PROMOTED_RC_TAG" "--promoted-rc-tag"
fi
if [[ ! -f "${ARTIFACT_DIR}/${CHECKSUM_NAME}" ]]; then
  echo "Checksum file not found: ${ARTIFACT_DIR}/${CHECKSUM_NAME}" >&2
  exit 1
fi

if ! target_commit="$(git rev-parse -q --verify "${TARGET_SHA}^{commit}" 2>/dev/null)"; then
  echo "Target commit does not resolve: $TARGET_SHA" >&2
  exit 1
fi
TARGET_SHA="$target_commit"

mkdir -p "$(dirname "$OUT_FILE")"
git fetch --tags --force >/dev/null 2>&1 || true

find_previous_stable_tag() {
  local exclude_tag="$1"
  local target="$2"
  local tag commit distance best_tag="" best_distance=""
  while IFS= read -r tag; do
    if [[ "$tag" == "$exclude_tag" || ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      continue
    fi
    if ! commit="$(git rev-list -n 1 "$tag" 2>/dev/null)"; then
      continue
    fi
    if [[ "$commit" == "$target" ]]; then
      continue
    fi
    if git merge-base --is-ancestor "$commit" "$target"; then
      distance="$(git rev-list --count "${commit}..${target}")"
      if [[ -z "$best_distance" ]] || (( distance < best_distance )); then
        best_tag="$tag"
        best_distance="$distance"
      fi
    fi
  done < <(git tag -l 'v[0-9]*.[0-9]*.[0-9]*')
  if [[ -n "$best_tag" ]]; then
    printf '%s\n' "$best_tag"
  fi
}

find_previous_rc_tag() {
  local target_version="$1"
  local rc_number="$2"
  local target="$3"
  local tag suffix best_tag="" best_number=0 commit
  while IFS= read -r tag; do
    suffix="${tag##*.}"
    if [[ ! "$suffix" =~ ^[0-9]+$ ]]; then
      continue
    fi
    if (( suffix >= rc_number || suffix <= best_number )); then
      continue
    fi
    if ! commit="$(git rev-list -n 1 "$tag" 2>/dev/null)"; then
      continue
    fi
    if ! git merge-base --is-ancestor "$commit" "$target"; then
      continue
    fi
    best_tag="$tag"
    best_number="$suffix"
  done < <(git tag -l "v${target_version}-rc.*")
  if [[ -n "$best_tag" ]]; then
    printf '%s\n' "$best_tag"
  fi
}

bounded_range_for_target() {
  local target="$1"
  local first_in_window parent
  first_in_window="$(git rev-list --max-count=80 "$target" | tail -n 1)"
  if parent="$(git rev-parse "${first_in_window}^" 2>/dev/null)"; then
    printf '%s..%s\n' "$parent" "$target"
  else
    printf '%s\n' "$target"
  fi
}

previous_ref=""
previous_label=""
case "$CHANNEL" in
  rc)
    previous_ref="$(find_previous_rc_tag "$TARGET_VERSION" "$RC_NUMBER" "$TARGET_SHA" || true)"
    if [[ -n "$previous_ref" ]]; then
      previous_label="$previous_ref"
    else
      previous_ref="$(find_previous_stable_tag "" "$TARGET_SHA" || true)"
      if [[ -n "$previous_ref" ]]; then
        previous_label="$previous_ref"
      fi
    fi
    ;;
  stable)
    previous_ref="$(find_previous_stable_tag "$RELEASE_TAG" "$TARGET_SHA" || true)"
    if [[ -n "$previous_ref" ]]; then
      previous_label="$previous_ref"
    fi
    ;;
esac

range=""
if [[ -n "$previous_ref" ]]; then
  previous_commit="$(git rev-list -n 1 "$previous_ref")"
  if [[ "$previous_commit" == "$TARGET_SHA" ]]; then
    range=""
  else
    range="${previous_commit}..${TARGET_SHA}"
  fi
else
  range="$(bounded_range_for_target "$TARGET_SHA")"
  previous_label="bounded commit window"
fi

changes_file="$(mktemp)"
artifacts_file="$(mktemp)"
cleanup() {
  rm -f "$changes_file" "$artifacts_file"
}
trap cleanup EXIT

zip_files=()
while IFS= read -r zip_name; do
  zip_files+=("$zip_name")
done < <(find "$ARTIFACT_DIR" -maxdepth 1 -type f -name '*.zip' -exec basename {} \; | sort)
if [[ ${#zip_files[@]} -eq 0 ]]; then
  echo "No release zip artifacts found in $ARTIFACT_DIR" >&2
  exit 1
fi
for zip_name in "${zip_files[@]}"; do
  stem="${zip_name%.zip}"
  arch="${stem##*-}"
  platform_part="${stem%-*}"
  platform="${platform_part##*-}"
  printf -- '- %s / %s: `%s`\n' "$platform" "$arch" "$zip_name" >> "$artifacts_file"
done

if [[ -z "$range" ]]; then
  echo "- No code changes since the previous release boundary; this run refreshed release artifacts from the same target commit." > "$changes_file"
elif [[ "${WAVECRATE_RELEASE_LOG_DISABLE_GIT_CLIFF:-0}" != "1" && -f "$CLIFF_CONFIG" && "$(command -v git-cliff || true)" ]]; then
  if ! git cliff --config "$CLIFF_CONFIG" --tag "$VERSION" "$range" > "$changes_file"; then
    git log --no-merges --pretty=format:'- %s' "$range" > "$changes_file"
  fi
else
  git log --no-merges --pretty=format:'- %s' "$range" > "$changes_file"
fi
if ! grep -Eq '^[[:space:]]*-' "$changes_file"; then
  git log --no-merges --pretty=format:'- %s' "$range" > "$changes_file" || true
fi
if [[ ! -s "$changes_file" ]]; then
  echo "- No commit messages were found for this release range." > "$changes_file"
fi

manual_notes="${RELEASE_NOTES:-}"
channel_label="Stable"
if [[ "$CHANNEL" == "rc" ]]; then
  channel_label="Release Candidate"
fi

{
  echo "# Wavecrate ${VERSION}"
  echo
  echo "## Release Metadata"
  echo
  echo "- Channel: ${channel_label}"
  echo "- Version: ${VERSION}"
  echo "- Target branch: ${TARGET_BRANCH}"
  echo "- Commit: ${TARGET_SHA}"
  echo "- Build date: ${BUILD_DATE}"
  if [[ "$CHANNEL" == "rc" ]]; then
    echo "- RC number: ${RC_NUMBER}"
  fi
  if [[ "$CHANNEL" == "stable" ]]; then
    echo "- Promoted from: ${PROMOTED_RC_TAG}"
  fi
  echo "- Previous release boundary: ${previous_label}"
  echo
  echo "## Artifacts"
  echo
  cat "$artifacts_file"
  echo
  echo "## Checksums"
  echo
  echo "- Checksums: \`${CHECKSUM_NAME}\`"
  echo "- Signature: \`${CHECKSUM_SIG_NAME}\`"
  echo
  if [[ -n "$manual_notes" ]]; then
    echo "## Manual Notes"
    echo
    printf '%s\n' "$manual_notes"
    echo
  fi
  echo "## Generated Changes"
  echo
  cat "$changes_file"
} > "$OUT_FILE"

if [[ ! -s "$OUT_FILE" ]]; then
  echo "Generated release log is empty." >&2
  exit 1
fi
