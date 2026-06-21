#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dir="$(cd "$script_dir/../../.." && pwd)"
radiant_dev_app="$root_dir/vendor/radiant/scripts/dev_app_bundle.sh"

cargo_package_version() {
  awk -F '"' '/^version =/ { print $2; exit }' "$root_dir/Cargo.toml"
}

falsey() {
  case "${1:-}" in
    0|false|FALSE|no|NO|off|OFF) return 0 ;;
    *) return 1 ;;
  esac
}

cd "$root_dir"

if [[ ! -x "$radiant_dev_app" ]]; then
  echo "[run][error] Missing executable Radiant dev app helper: $radiant_dev_app" >&2
  exit 2
fi

cargo build --release --bin wavecrate

export RADIANT_DEV_APP_NAME="Wavecrate"
export RADIANT_DEV_APP_BINARY="$root_dir/target/release/wavecrate"
export RADIANT_DEV_APP_BUNDLE_ID="${WAVECRATE_DEV_BUNDLE_ID:-com.portalsurfer.wavecrate.dev}"
export RADIANT_DEV_APP_BUNDLE_ROOT="${WAVECRATE_DEV_APP_DIR:-$root_dir/target/dev-app}"
export RADIANT_DEV_APP_VERSION="${WAVECRATE_DEV_APP_VERSION:-$(cargo_package_version)}"
export RADIANT_DEV_APP_CATEGORY="${WAVECRATE_DEV_APP_CATEGORY:-public.app-category.music}"
export RADIANT_DEV_APP_ICON="${WAVECRATE_DEV_APP_ICON:-$root_dir/assets/logo3.icns}"
export RADIANT_DEV_APP_DOCUMENT_TYPE_NAME="${WAVECRATE_DEV_APP_DOCUMENT_TYPE_NAME:-Waveform audio}"
export RADIANT_DEV_APP_DOCUMENT_EXTENSIONS="${WAVECRATE_DEV_APP_DOCUMENT_EXTENSIONS:-wav wave}"
export RADIANT_DEV_APP_DOCUMENT_CONTENT_TYPES="${WAVECRATE_DEV_APP_DOCUMENT_CONTENT_TYPES:-com.microsoft.waveform-audio public.wav}"
export RADIANT_DEV_APP_DOCUMENT_ROLE="${WAVECRATE_DEV_APP_DOCUMENT_ROLE:-Editor}"
export RADIANT_DEV_APP_DOCUMENT_HANDLER_RANK="${WAVECRATE_DEV_APP_DOCUMENT_HANDLER_RANK:-Alternate}"
export RADIANT_DEV_APP_PREPARE_ONLY="${WAVECRATE_DEV_APP_PREPARE_ONLY:-}"

if ! falsey "${WAVECRATE_AUTOMATION_TARGET_EXPORT:-1}" \
  && [[ -z "${RADIANT_AUTOMATION_TARGET_EXPORT:-}" ]]; then
  export RADIANT_AUTOMATION_TARGET_EXPORT="${WAVECRATE_AUTOMATION_TARGETS_PATH:-$root_dir/target/dev-app/Wavecrate.automation-targets.json}"
  export RADIANT_AUTOMATION_TARGET_EXPORT_PRETTY="${WAVECRATE_AUTOMATION_TARGETS_PRETTY:-1}"
fi

exec "$radiant_dev_app" --log "$@"
