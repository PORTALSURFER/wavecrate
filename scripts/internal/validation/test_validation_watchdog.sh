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
export WAVECRATE_VALIDATION_TEST_PLATFORM=Linux
wavecrate_use_validation_target "$ROOT_DIR"
[[ -z "${CARGO_TARGET_DIR:-}" ]] || fail "non-Darwin target selection changed CARGO_TARGET_DIR"
echo "[validation_watchdog_test] PASS: non-Darwin target selection is a no-op"

export WAVECRATE_VALIDATION_TEST_PLATFORM=Darwin
wavecrate_use_validation_target "$ROOT_DIR"
first_target="$CARGO_TARGET_DIR"
wavecrate_release_validation_target
mkdir -p "$first_target/debug/deps"
: >"$first_target/debug/deps/artifact"

run_target_selection() {
  local output_path="$1"
  unset CARGO_TARGET_DIR
  wavecrate_use_validation_target "$ROOT_DIR"
  printf '%s\n' "$CARGO_TARGET_DIR" > "$output_path"
  sleep 0.2
  wavecrate_release_validation_target
}

run_target_selection "$FIXTURE_DIR/target-one" &
first_selector=$!
run_target_selection "$FIXTURE_DIR/target-two" &
second_selector=$!
wait "$first_selector"
wait "$second_selector"
[[ "$(cat "$FIXTURE_DIR/target-one")" == "$first_target" ]] \
  || fail "first concurrent target identity changed"
[[ "$(cat "$FIXTURE_DIR/target-two")" == "$first_target" ]] \
  || fail "second concurrent target identity changed"
quarantine_count="$(find "$target_root" -mindepth 1 -maxdepth 1 -type d -name 'stale-*' | wc -l | tr -d ' ')"
[[ "$quarantine_count" == "1" ]] || fail "expected one quarantine, got $quarantine_count"
find "$target_root" -mindepth 1 -maxdepth 1 -type d -name '.lock-*' | grep -q . \
  && fail "validation target lease survived successful selectors"
echo "[validation_watchdog_test] PASS: concurrent pathological target rotation"

unset WAVECRATE_VALIDATION_TEST_PLATFORM
unset CARGO_TARGET_DIR
runner_target_root="$FIXTURE_DIR/runner-targets"
WAVECRATE_VALIDATION_TARGET_ROOT="$runner_target_root" \
  "$ROOT_DIR/scripts/internal/validation/run_validation_command.sh" sh -c 'sleep 0.2' &
first_runner=$!
WAVECRATE_VALIDATION_TARGET_ROOT="$runner_target_root" \
  "$ROOT_DIR/scripts/internal/validation/run_validation_command.sh" sh -c 'sleep 0.2' &
second_runner=$!
wait "$first_runner"
wait "$second_runner"
find "$runner_target_root" -mindepth 1 -maxdepth 1 -type d -name '.lock-*' | grep -q . \
  && fail "validation command left its target lease behind"
echo "[validation_watchdog_test] PASS: concurrent validation command leases"

echo "[validation_watchdog_test] OK"
