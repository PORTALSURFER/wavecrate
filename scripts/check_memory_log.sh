#!/usr/bin/env bash

# Validates that MEMORY.md has a fresh "Last Updated" timestamp and updater marker.
#
# This is an agent-handoff guardrail: if the file is stale or misses the
# expected updater identity, local CI should fail fast.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MEMORY_FILE="MEMORY.md"
MAX_AGE_HOURS=24
REQUIRED_UPDATER="Codex"

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

if ! updated_epoch="$(date -u -d "$timestamp" +%s 2>/dev/null)"; then
  echo "[memory_log] Failed to parse timestamp in MEMORY.md: $timestamp" >&2
  exit 1
fi

if [[ ! "$updated_by_line" =~ ^Updated[[:space:]]By:\ (.+)$ ]]; then
  echo "[memory_log] 'Updated By:' line malformed. Expected format: Updated By: Codex" >&2
  exit 1
fi
updated_by="${BASH_REMATCH[1]}"

if [[ "$updated_by" != "$REQUIRED_UPDATER" ]]; then
  echo "[memory_log] MEMORY.md must be updated by '$REQUIRED_UPDATER'. Found: $updated_by" >&2
  exit 1
fi

now_epoch="$(date -u +%s)"
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

echo "[memory_log] OK (${timestamp} by $updated_by)"
