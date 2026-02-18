#!/usr/bin/env bash

# Validate run contract artifacts (manifest + NDJSON milestones) for deterministic
# assertions in automated harness checks.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check_run_contract_smoke.sh --artifact <run_contract.ndjson> [--manifest <run_manifest.json>]
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

artifact=""
manifest=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact)
      if [[ $# -lt 2 ]]; then
        echo "error: --artifact requires a path" >&2
        usage
        exit 2
      fi
      artifact="$2"
      shift 2
      ;;
    --manifest)
      if [[ $# -lt 2 ]]; then
        echo "error: --manifest requires a path" >&2
        usage
        exit 2
      fi
      manifest="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$artifact" ]]; then
  echo "error: --artifact is required" >&2
  usage
  exit 2
fi

if [[ ! -f "$artifact" ]]; then
  echo "error: missing artifact '$artifact'" >&2
  exit 1
fi

if [[ -z "$manifest" ]]; then
  manifest_name="$(basename "$artifact")"
  manifest_name="${manifest_name/run_contract_/run_manifest_}"
  if [[ "$manifest_name" == *.ndjson ]]; then
    manifest_name="${manifest_name%.ndjson}.json"
  fi
  manifest_dir="$(dirname "$artifact")"
  manifest="$manifest_dir/$manifest_name"
fi

if [[ ! -f "$manifest" ]]; then
  echo "error: missing manifest '$manifest'" >&2
  exit 1
fi

python3 - "$artifact" "$manifest" <<'PY'
import json
import pathlib
import sys

artifact_path = pathlib.Path(sys.argv[1])
manifest_path = pathlib.Path(sys.argv[2])

try:
    artifact_text = artifact_path.read_text(encoding="utf-8").splitlines()
except OSError as err:
    print(f"error: failed to read artifact '{artifact_path}': {err}", file=sys.stderr)
    sys.exit(1)

events = []
for line_num, line in enumerate(artifact_text, 1):
    line = line.strip()
    if not line:
        continue
    try:
        events.append(json.loads(line))
    except json.JSONDecodeError as err:
        print(
            f"error: malformed JSON on artifact line {line_num} for {artifact_path}: {err}",
            file=sys.stderr,
        )
        sys.exit(1)

if not events:
    print(f"error: no events found in {artifact_path}", file=sys.stderr)
    sys.exit(1)

required_event_fields = {
    "run_id",
    "manifest_path",
    "artifact_path",
    "startup_phase",
    "milestone",
    "exit_status",
    "timestamp_utc",
}

run_id = events[0]["run_id"]
for idx, event in enumerate(events, 1):
    missing = [field for field in required_event_fields if field not in event]
    if missing:
        print(
            f"error: event {idx} missing required field(s) {missing} in {artifact_path}",
            file=sys.stderr,
        )
        sys.exit(1)
    if event["run_id"] != run_id:
        print(
            f"error: run_id mismatch on event {idx}: {event['run_id']} != {run_id}",
            file=sys.stderr,
        )
        sys.exit(1)

required_milestones = ["startup_begin", "runtime_started", "runtime_exit"]
failure_milestones = ["startup_begin", "startup_failed"]
seen = []
for event in events:
    seen.append(event["milestone"])

if "startup_failed" in seen:
    if "runtime_started" in seen:
        print(
            f"error: startup_failed and runtime_started cannot both be present in {artifact_path}",
            file=sys.stderr,
        )
        sys.exit(1)
    required_milestones = failure_milestones

for milestone in required_milestones:
    if milestone not in seen:
        print(
            f"error: required milestone '{milestone}' missing from artifact {artifact_path}",
            file=sys.stderr,
        )
        sys.exit(1)

# Deterministic sequence check for startup -> runtime -> exit.
required_indexes = [seen.index(name) for name in required_milestones]
if required_indexes != sorted(required_indexes):
    print(
        f"error: milestones are out of order in {artifact_path}: {seen}",
        file=sys.stderr,
    )
    sys.exit(1)

if "startup_failed" in seen and seen[-1] != "startup_failed":
    print(
        f"error: startup-failed run {artifact_path} must end with startup_failed",
        file=sys.stderr,
    )
    sys.exit(1)

timestamps = []
for idx, event in enumerate(events, 1):
    if event.get("manifest_path") != str(manifest_path.resolve()):
        print(
            f"error: event {idx} manifest_path {event.get('manifest_path')} does not match manifest {manifest_path}",
            file=sys.stderr,
        )
        sys.exit(1)
    if event.get("artifact_path") != str(artifact_path.resolve()):
        print(
            f"error: event {idx} artifact_path {event.get('artifact_path')} does not match artifact {artifact_path}",
            file=sys.stderr,
        )
        sys.exit(1)

    if not isinstance(event.get("timestamp_utc"), str):
        print(
            f"error: timestamp_utc must be string on event {idx} in {artifact_path}",
            file=sys.stderr,
        )
        sys.exit(1)
    try:
        timestamps.append(int(event["timestamp_utc"]))
    except ValueError:
        print(
            f"error: non-numeric timestamp_utc on event {idx} in {artifact_path}: {event['timestamp_utc']}",
            file=sys.stderr,
        )
        sys.exit(1)

if any(current < previous for previous, current in zip(timestamps, timestamps[1:])):
    print(
        f"error: timestamp_utc is not monotonic in artifact {artifact_path}",
        file=sys.stderr,
    )
    sys.exit(1)

try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as err:
    print(f"error: failed to load manifest '{manifest_path}': {err}", file=sys.stderr)
    sys.exit(1)

required_manifest_fields = {
    "run_id",
    "artifact_path",
    "manifest_path",
    "milestones",
    "exit_status",
}

missing_manifest = sorted(field for field in required_manifest_fields if field not in manifest)
if missing_manifest:
    print(
        f"error: manifest missing required fields {missing_manifest}: {manifest_path}",
        file=sys.stderr,
    )
    sys.exit(1)

if manifest.get("run_id") != run_id:
    print(
        f"error: manifest run_id {manifest.get('run_id')} does not match artifact run_id {run_id}",
        file=sys.stderr,
    )
    sys.exit(1)

manifest_artifact_path = manifest.get("artifact_path")
manifest_path_field = manifest.get("manifest_path")
if manifest_artifact_path != str(artifact_path.resolve()):
    print(
        f"error: manifest artifact_path {manifest_artifact_path} does not match artifact {artifact_path}",
        file=sys.stderr,
    )
    sys.exit(1)

if manifest_path_field != str(manifest_path.resolve()):
    print(
        f"error: manifest manifest_path {manifest_path_field} does not match manifest {manifest_path}",
        file=sys.stderr,
    )
    sys.exit(1)

final_event_status = events[-1].get("exit_status")
if final_event_status != manifest.get("exit_status"):
    print(
        f"error: final artifact status {final_event_status} does not match manifest status {manifest.get('exit_status')}",
        file=sys.stderr,
    )
    sys.exit(1)

if final_event_status not in {"success", "error"}:
    print(
        f"error: final artifact status '{final_event_status}' is not one of {{success,error}}",
        file=sys.stderr,
    )
    sys.exit(1)


manifest_milestones = manifest.get("milestones")
if not isinstance(manifest_milestones, list):
    print(
        f"error: manifest milestones must be an array in {manifest_path}",
        file=sys.stderr,
    )
    sys.exit(1)

manifest_names = [milestone.get("name") for milestone in manifest_milestones]
if any(name is None for name in manifest_names):
    print(
        f"error: manifest milestones must include name for every entry in {manifest_path}",
        file=sys.stderr,
    )
    sys.exit(1)

for milestone in required_milestones:
    if milestone not in manifest_names:
        print(
            f"error: manifest missing required milestone '{milestone}' in {manifest_path}",
            file=sys.stderr,
        )
        sys.exit(1)

manifest_indexes = [manifest_names.index(name) for name in required_milestones]
if manifest_indexes != sorted(manifest_indexes):
    print(
        f"error: manifest milestones are out of order in {manifest_path}: {manifest_names}",
        file=sys.stderr,
    )
    sys.exit(1)

artifact_required = [seen[index] for index in required_indexes]
manifest_required = [manifest_names[index] for index in manifest_indexes]
if artifact_required != manifest_required:
    print(
        f"error: required milestone order mismatch between artifact and manifest in {manifest_path}",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"check_run_contract_smoke: ok ({artifact_path}, run_id={run_id})")
PY
