#!/usr/bin/env bash

# Validates that MEMORY.md has a fresh "Last Updated" timestamp and updater marker.
#
# This is an agent-handoff guardrail: if the file is stale or malformed, local CI
# should fail fast.
#
# `MEMORY_MAX_AGE_HOURS` and `MEMORY_REQUIRED_UPDATER` are read from the
# environment. Use `MEMORY_REQUIRED_UPDATER` to enforce an exact match on the
# `Updated By:` line. Leave it unset for team-wide/local checks.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MEMORY_FILE="MEMORY.md"
MAX_AGE_HOURS="${MEMORY_MAX_AGE_HOURS:-24}"
REQUIRED_UPDATER="${MEMORY_REQUIRED_UPDATER:-}"

if ! [[ "$MAX_AGE_HOURS" =~ ^[0-9]+$ ]]; then
  echo "[memory_log] MEMORY_MAX_AGE_HOURS must be a non-negative integer; got: $MAX_AGE_HOURS" >&2
  exit 1
fi

if [[ ! -f "$MEMORY_FILE" ]]; then
  echo "[memory_log] Missing required file: $MEMORY_FILE" >&2
  exit 1
fi

last_updated_line="$(awk '/^Last Updated:/ {print; exit}' "$MEMORY_FILE" || true)"
updated_by_line="$(awk '/^Updated By:/ {print; exit}' "$MEMORY_FILE" || true)"

if [[ -z "$last_updated_line" ]]; then
  echo "[memory_log] MEMORY.md missing 'Last Updated:' line." >&2
  exit 1
fi

if [[ -z "$updated_by_line" ]]; then
  echo "[memory_log] MEMORY.md missing 'Updated By:' line." >&2
  exit 1
fi

if [[ ! "$last_updated_line" =~ ^Last[[:space:]]Updated:\ ([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z)$ ]]; then
  echo "[memory_log] 'Last Updated:' must be ISO-8601 UTC, e.g. 2026-02-18T12:06:16Z." >&2
  exit 1
fi
timestamp="${BASH_REMATCH[1]}"

if [[ ! "$updated_by_line" =~ ^Updated[[:space:]]By:\ (.+)$ ]]; then
  echo "[memory_log] 'Updated By:' line malformed. Expected format: Updated By: <name>" >&2
  exit 1
fi
updated_by="${BASH_REMATCH[1]}"

if [[ -n "$REQUIRED_UPDATER" && "$updated_by" != "$REQUIRED_UPDATER" ]]; then
  echo "[memory_log] MEMORY.md must be updated by '$REQUIRED_UPDATER'. Found: $updated_by" >&2
  exit 1
fi

now_epoch="$(date -u +%s)"

parse_iso_utc_epoch() {
  local timestamp="$1"
  local parsed_epoch=""

  if command -v python3 >/dev/null 2>&1; then
    parsed_epoch="$(python3 - "$timestamp" <<'PY'
import datetime
import sys

ts = sys.argv[1]
dt = datetime.datetime.strptime(ts, "%Y-%m-%dT%H:%M:%SZ")
print(int(dt.replace(tzinfo=datetime.timezone.utc).timestamp()))
PY
)" || true
  elif command -v ruby >/dev/null 2>&1; then
    parsed_epoch="$(ruby -e 'require "time"; ts=ARGV[0]; puts Time.parse(ts).utc.to_i' "$timestamp")" || true
  else
    return 1
  fi

  if [[ -z "$parsed_epoch" ]]; then
    return 1
  fi

  if ! [[ "$parsed_epoch" =~ ^[0-9]+$ ]]; then
    return 1
  fi

  printf '%s\n' "$parsed_epoch"
}

if ! updated_epoch="$(parse_iso_utc_epoch "$timestamp")"; then
  echo "[memory_log] Failed to parse timestamp in MEMORY.md: $timestamp" >&2
  exit 1
fi
age_seconds=$(( now_epoch - updated_epoch ))

if (( age_seconds < 0 )); then
  echo "[memory_log] MEMORY.md timestamp is in the future: $timestamp" >&2
  exit 1
fi

max_age_seconds=$(( MAX_AGE_HOURS * 60 * 60 ))
if (( age_seconds > max_age_seconds )); then
  hours=$(( age_seconds / 3600 ))
  echo "[memory_log] MEMORY.md is too stale. Last update: $timestamp (${hours}h ago)." >&2
  exit 1
fi

echo "[memory_log] OK (${timestamp} by $updated_by, max_age=${MAX_AGE_HOURS}h)"
