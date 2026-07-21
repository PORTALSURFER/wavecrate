#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
WATCHDOG="$ROOT_DIR/scripts/internal/validation/run_with_progress_watchdog.py"
FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "$FIXTURE_DIR"' EXIT

fail() {
  echo "[validation_watchdog_test] FAIL: $*" >&2
  exit 1
}

run_watchdog() {
  WAVECRATE_VALIDATION_IDLE_SECONDS=1 \
    WAVECRATE_VALIDATION_DIAGNOSTIC_GRACE_SECONDS=1 \
    WAVECRATE_VALIDATION_TERM_GRACE_SECONDS=1 \
    WAVECRATE_VALIDATION_POLL_SECONDS=0.1 \
    WAVECRATE_VALIDATION_SAMPLE_SECONDS=0 \
    WAVECRATE_VALIDATION_DIAGNOSTICS_DIR="$FIXTURE_DIR/diagnostics" \
    python3 "$WATCHDOG" "$@"
}

run_watchdog sh -c 'exit 0'
echo "[validation_watchdog_test] PASS: successful command"

set +e
run_watchdog sh -c 'exit 23'
status=$?
set -e
[[ "$status" == "23" ]] || fail "expected child exit 23, got $status"
echo "[validation_watchdog_test] PASS: child exit propagation"

orphan_pid_file="$FIXTURE_DIR/orphan-child.pid"
run_watchdog sh -c "sleep 30 & echo \$! > '$orphan_pid_file'"
orphan_pid="$(cat "$orphan_pid_file")"
kill -0 "$orphan_pid" 2>/dev/null && fail "background child survived normal parent exit"
echo "[validation_watchdog_test] PASS: normal-exit child cleanup"

run_watchdog python3 -c 'import time; end=time.monotonic()+1.5
while time.monotonic() < end: pass'
echo "[validation_watchdog_test] PASS: CPU progress is not classified as a stall"

sleep 30 &
unrelated_pid=$!
set +e
run_watchdog sh -c 'sleep 30' >"$FIXTURE_DIR/stall.out" 2>&1
status=$?
set -e
[[ "$status" == "124" ]] || fail "expected stall exit 124, got $status"
kill -0 "$unrelated_pid" 2>/dev/null || fail "unrelated process was terminated"
kill "$unrelated_pid"
wait "$unrelated_pid" 2>/dev/null || true
grep -Fq "confirmed no progress" "$FIXTURE_DIR/stall.out" || fail "missing stall diagnostic"
diagnostic_dir="$(find "$FIXTURE_DIR/diagnostics" -mindepth 1 -maxdepth 1 -type d | head -1)"
[[ -f "$diagnostic_dir/process-tree.tsv" ]] || fail "missing process tree"
[[ -f "$diagnostic_dir/command.txt" ]] || fail "missing command record"
echo "[validation_watchdog_test] PASS: stall diagnostics and owned cleanup"

child_pid_file="$FIXTURE_DIR/cancel-child.pid"
WAVECRATE_VALIDATION_IDLE_SECONDS=30 \
  WAVECRATE_VALIDATION_DIAGNOSTIC_GRACE_SECONDS=1 \
  WAVECRATE_VALIDATION_TERM_GRACE_SECONDS=1 \
  WAVECRATE_VALIDATION_POLL_SECONDS=0.1 \
  python3 "$WATCHDOG" sh -c "sleep 30 & echo \$! > '$child_pid_file'; wait" &
wrapper_pid=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do
  [[ -s "$child_pid_file" ]] && break
  sleep 0.1
done
[[ -s "$child_pid_file" ]] || fail "cancellation fixture did not start"
owned_child_pid="$(cat "$child_pid_file")"
kill -TERM "$wrapper_pid"
set +e
wait "$wrapper_pid"
status=$?
set -e
[[ "$status" == "143" ]] || fail "expected cancellation exit 143, got $status"
kill -0 "$owned_child_pid" 2>/dev/null && fail "owned child survived cancellation"
echo "[validation_watchdog_test] PASS: cancellation cleanup"

target_root="$FIXTURE_DIR/targets"
export WAVECRATE_VALIDATION_TARGET_ROOT="$target_root"
export WAVECRATE_VALIDATION_MAX_DEPS_METADATA_BYTES=1
unset CARGO_TARGET_DIR
# shellcheck source=scripts/internal/validation/use_validation_target.sh
source "$ROOT_DIR/scripts/internal/validation/use_validation_target.sh"
wavecrate_use_validation_target "$ROOT_DIR"
first_target="$CARGO_TARGET_DIR"
mkdir -p "$first_target/debug/deps"
: >"$first_target/debug/deps/artifact"
unset CARGO_TARGET_DIR
wavecrate_use_validation_target "$ROOT_DIR"
[[ "$CARGO_TARGET_DIR" == "$first_target" ]] || fail "target identity changed during rotation"
find "$target_root" -mindepth 1 -maxdepth 1 -type d -name 'stale-*' | grep -q . \
  || fail "pathological target was not quarantined"
echo "[validation_watchdog_test] PASS: pathological target rotation"

echo "[validation_watchdog_test] OK"
