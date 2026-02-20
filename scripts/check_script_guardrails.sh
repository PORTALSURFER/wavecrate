#!/usr/bin/env bash

# Guardrail checks for scripts used by the agent harness.
#
# This script verifies:
# - shell syntax checks for key scripts
# - fixture-based checks for `.rs` matching in file-size budget logic
# - fixture-based checks for diff-aware matching in taste invariants

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0

run_expect_exit_code() {
  local label="$1"
  local expected_code="$2"
  local work_dir="$3"
  shift 3

  local output_file
  output_file="$(mktemp)"
  set +e
  (cd "$work_dir" && "$@") >"$output_file" 2>&1
  local exit_code=$?
  set -e

  if (( exit_code == expected_code )); then
    echo "[guardrails] PASS: $label"
  else
    echo "[guardrails] FAIL: $label (expected $expected_code, got $exit_code)" >&2
    cat "$output_file" >&2
    failures=$((failures + 1))
  fi
  rm -f "$output_file"
}

run_file_size_budget_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/check_file_size_budget.sh"
  mkdir -p "$repo_dir/src" "$repo_dir/scripts"
  cp "scripts/check_file_size_budget.sh" "$script_path"
  chmod +x "$script_path"

  git -C "$fixture_dir/repo" init -q
  git -C "$fixture_dir/repo" config user.name "sempal-ci"
  git -C "$fixture_dir/repo" config user.email "ci@sempal.test"

  mkdir -p "$fixture_dir/repo/vendor/radiant/src"
  mkdir -p "$fixture_dir/repo/tests"
  cat >"$fixture_dir/repo/src/too_many_lines.rs" <<'EOF'
fn main() {
    println!("a");
    println!("b");
    println!("c");
    println!("d");
}
EOF
  cat >"$fixture_dir/repo/vendor/radiant/src/ok.rs" <<'EOF'
fn one() {}
EOF
  cat >"$fixture_dir/repo/tests/notes.txt" <<'EOF'
plain text should not be checked by script
EOF

  git -C "$fixture_dir/repo" add src/too_many_lines.rs
  git -C "$fixture_dir/repo" add vendor/radiant/src/ok.rs
  git -C "$fixture_dir/repo" add tests/notes.txt
  git -C "$fixture_dir/repo" commit -qm "seed"

  run_expect_exit_code \
    "file-size-budget fixture catches over-limit .rs file" \
    1 \
    "$repo_dir" \
    "$script_path" \
    --all \
    --limit \
    3

  run_expect_exit_code \
    "file-size-budget fixture passes when under limit" \
    0 \
    "$repo_dir" \
    "$script_path" \
    --all \
    --limit \
    10

  local collection_trace
  collection_trace="$(mktemp)"
  set +e
  (cd "$repo_dir" && "$script_path" --all --limit 3) >"$collection_trace" 2>&1
  local collection_code=$?
  set -e

  if [[ "$collection_code" -ne 1 ]]; then
    echo "[guardrails] FAIL: file-size-budget fixture collected-file-count trace requires failure exit code 1 (got $collection_code)" >&2
    cat "$collection_trace" >&2
    failures=$((failures + 1))
  else
    local collection_count
    collection_count="$(grep -oE 'collected_file_count=[0-9]+' "$collection_trace" | tail -n 1 | cut -d= -f2)"
    if [[ -z "$collection_count" ]]; then
      echo "[guardrails] FAIL: file-size-budget fixture did not emit collected_file_count trace" >&2
      cat "$collection_trace" >&2
      failures=$((failures + 1))
    elif (( collection_count < 2 )); then
      echo "[guardrails] FAIL: file-size-budget fixture collected_file_count=$collection_count, below minimum threshold 2" >&2
      cat "$collection_trace" >&2
      failures=$((failures + 1))
    else
      echo "[guardrails] PASS: file-size-budget fixture emitted collected_file_count=$collection_count"
    fi
  fi
  rm -f "$collection_trace"

  run_expect_exit_code \
    "file-size-budget fixture treats --base with missing value as parse error" \
    2 \
    "$repo_dir" \
    "$script_path" \
    --base

  run_expect_exit_code \
    "file-size-budget fixture treats --limit with non-numeric value as parse error" \
    2 \
    "$repo_dir" \
    "$script_path" \
    --limit \
    invalid \
    --all
}

run_taste_invariants_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/check_rust_taste_invariants.sh"
  mkdir -p "$repo_dir/src" "$repo_dir/scripts"
  cp "scripts/check_rust_taste_invariants.sh" "$script_path"
  chmod +x "$script_path"

  git -C "$repo_dir" init -q
  git -C "$repo_dir" config user.name "sempal-ci"
  git -C "$repo_dir" config user.email "ci@sempal.test"

  cat >"$repo_dir/src/lib.rs" <<'EOF'
fn main() {
    let _value = 1;
}
EOF
  git -C "$repo_dir" add src/lib.rs
  git -C "$repo_dir" commit -qm "seed"

  # Add a violation and ensure the check fails.
  cat >>"$repo_dir/src/lib.rs" <<'EOF'

fn debug() {
    println!("guardrail");
}
EOF

  run_expect_exit_code \
    "taste invariants fixture detects added violation" \
    1 \
    "$repo_dir" \
    "$script_path"

  # Fix the file and ensure the script exits cleanly.
  cat >"$repo_dir/src/lib.rs" <<'EOF'
fn main() {
    let _value = 1;
}

fn debug() {
    let _ = 1;
}
EOF

  run_expect_exit_code \
    "taste invariants fixture passes when violations are absent" \
    0 \
    "$repo_dir" \
    "$script_path"

  run_expect_exit_code \
    "taste invariants fixture treats --base with missing value as parse error" \
    2 \
    "$repo_dir" \
    "$script_path" \
    --base

  run_expect_exit_code \
    "taste invariants fixture treats --head with missing value as parse error" \
    2 \
    "$repo_dir" \
    "$script_path" \
    --head

  cat >"$repo_dir/src/lib.rs" <<'EOF'
// println!("guardrail");
fn x() {
    // println!("guardrail");
    let _ = 1;
}
EOF

  run_expect_exit_code \
    "taste invariants fixture ignores debug prints in comment-only lines" \
    0 \
    "$repo_dir" \
    "$script_path" \
    --base \
    HEAD
}

run_run_contract_smoke_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local fixture_script="$ROOT_DIR/scripts/check_run_contract_smoke.sh"
  local artifact="$fixture_dir/run_contract_123-456.ndjson"
  local manifest="$fixture_dir/run_manifest_123-456.json"
  local bad_artifact="$fixture_dir/run_contract_bad.json"
  local bad_manifest="$fixture_dir/run_manifest_bad.json"
  local startup_fail_artifact="$fixture_dir/run_contract_startup_fail.ndjson"
  local startup_fail_manifest="$fixture_dir/run_manifest_startup_fail.json"
  local valid_manifest_path="$fixture_dir/run_manifest_123-456.json"
  local bad_manifest_path="$fixture_dir/run_manifest_bad.json"
  local valid_artifact_path="$fixture_dir/run_contract_123-456.ndjson"
  local bad_artifact_path="$fixture_dir/run_contract_bad.json"

  cat >"$artifact" <<EOF
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"startup","milestone":"startup_begin","exit_status":"running","timestamp_utc":"1","process_id":111,"manifest_path":"$valid_manifest_path","artifact_path":"$valid_artifact_path"}
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"runtime","milestone":"runtime_started","exit_status":"running","timestamp_utc":"2","process_id":111,"manifest_path":"$valid_manifest_path","artifact_path":"$valid_artifact_path"}
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"shutdown","milestone":"runtime_exit","exit_status":"success","timestamp_utc":"3","process_id":111,"manifest_path":"$valid_manifest_path","artifact_path":"$valid_artifact_path"}
EOF
  cat >"$manifest" <<EOF
{
  "run_id": "123-456",
  "artifact_path": "$valid_artifact_path",
  "manifest_path": "$valid_manifest_path",
  "milestones": [
    {"name": "startup_begin", "startup_phase": "startup", "status": "running", "timestamp_utc": "1"},
    {"name": "runtime_started", "startup_phase": "runtime", "status": "running", "timestamp_utc": "2"},
    {"name": "runtime_exit", "startup_phase": "shutdown", "status": "success", "timestamp_utc": "3"}
  ],
  "exit_status": "success"
}
EOF

  run_expect_exit_code \
    "run-contract smoke fixture passes for valid manifest+artifact" \
    0 \
    "$fixture_dir" \
    "$fixture_script" \
    --artifact \
    "$artifact"

  cat >"$bad_artifact" <<EOF
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"runtime","milestone":"runtime_started","exit_status":"running","timestamp_utc":"2","process_id":111,"manifest_path":"$bad_manifest_path","artifact_path":"$bad_artifact_path"}
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"startup","milestone":"startup_begin","exit_status":"running","timestamp_utc":"1","process_id":111,"manifest_path":"$bad_manifest_path","artifact_path":"$bad_artifact_path"}
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"shutdown","milestone":"runtime_exit","exit_status":"success","timestamp_utc":"3","process_id":111,"manifest_path":"$bad_manifest_path","artifact_path":"$bad_artifact_path"}
EOF

  run_expect_exit_code \
    "run-contract smoke fixture fails on invalid milestone order" \
    1 \
    "$fixture_dir" \
    "$fixture_script" \
    --artifact \
    "$bad_artifact" \
    --manifest \
    "$bad_manifest"

  cat >"$bad_manifest" <<EOF
{
  "run_id": "123-456",
  "artifact_path": "$bad_artifact_path",
  "manifest_path": "$bad_manifest_path",
  "milestones": [
    {"name": "runtime_started", "startup_phase": "runtime", "status": "running", "timestamp_utc": "2"},
    {"name": "startup_begin", "startup_phase": "startup", "status": "running", "timestamp_utc": "1"},
    {"name": "runtime_exit", "startup_phase": "shutdown", "status": "success", "timestamp_utc": "3"}
  ],
  "exit_status": "success"
}
EOF

  cat >"$startup_fail_artifact" <<EOF
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"startup","milestone":"startup_begin","exit_status":"running","timestamp_utc":"1","process_id":111,"manifest_path":"$startup_fail_manifest","artifact_path":"$startup_fail_artifact"}
{"run_id":"123-456","git_sha":"abc1234","cfg_path":"$fixture_dir","log_path":"$fixture_dir","startup_phase":"startup","milestone":"startup_failed","exit_status":"error","timestamp_utc":"2","process_id":111,"manifest_path":"$startup_fail_manifest","artifact_path":"$startup_fail_artifact"}
EOF
  cat >"$startup_fail_manifest" <<EOF
{
  "run_id": "123-456",
  "artifact_path": "$startup_fail_artifact",
  "manifest_path": "$startup_fail_manifest",
  "milestones": [
    {"name": "startup_begin", "startup_phase": "startup", "status": "running", "timestamp_utc": "1"},
    {"name": "startup_failed", "startup_phase": "startup", "status": "error", "timestamp_utc": "2"}
  ],
  "exit_status": "error"
}
EOF

  run_expect_exit_code \
    "run-contract smoke fixture passes for startup failure sequence" \
    0 \
    "$fixture_dir" \
    "$fixture_script" \
    --artifact \
    "$startup_fail_artifact" \
    --manifest \
    "$startup_fail_manifest"
}

run_quality_score_drift_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/check_quality_score_drift.sh"
  mkdir -p "$repo_dir/src" "$repo_dir/docs" "$repo_dir/scripts"
  cp "scripts/check_quality_score_drift.sh" "$script_path"
  cp "scripts/check_file_size_budget.sh" "$repo_dir/scripts/check_file_size_budget.sh"
  cp "scripts/check_rust_taste_invariants.sh" "$repo_dir/scripts/check_rust_taste_invariants.sh"
  chmod +x "$script_path"
  chmod +x "$repo_dir/scripts/check_file_size_budget.sh"
  chmod +x "$repo_dir/scripts/check_rust_taste_invariants.sh"

  git -C "$repo_dir" init -q
  git -C "$repo_dir" config user.name "sempal-ci"
  git -C "$repo_dir" config user.email "ci@sempal.test"

  cat >"$repo_dir/src/lib.rs" <<'EOF'
fn main() {
    let _value = 1;
}
EOF

  cat >"$repo_dir/docs/QUALITY_SCORE.md" <<'EOF'
# Quality Scorecard

| Area | Score | Notes |
| --- | ---: | --- |
| Agent-facing guardrails | 4 | Guardrails are currently healthy. |
EOF

  git -C "$repo_dir" add src/lib.rs docs/QUALITY_SCORE.md
  git -C "$repo_dir" commit -qm "seed"

  run_expect_exit_code \
    "quality score drift fixture passes when score matches healthy guardrails" \
    0 \
    "$repo_dir" \
    "$script_path"

  cat >>"$repo_dir/src/lib.rs" <<'EOF'

fn guarded() {
    println!("guardrail drift");
}
EOF

  run_expect_exit_code \
    "quality score drift fixture fails when score is still high while guardrails degrade" \
    1 \
    "$repo_dir" \
    "$script_path"
}

run_memory_log_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/check_memory_log.sh"
  mkdir -p "$repo_dir/scripts"
  cp "scripts/check_memory_log.sh" "$script_path"
  chmod +x "$script_path"

  local timestamp
  timestamp="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
  cat >"$repo_dir/MEMORY.md" <<EOF
# MEMORY
Last Updated: $timestamp
Updated By: Codex
EOF

  run_expect_exit_code \
    "memory log fixture passes without required updater" \
    0 \
    "$repo_dir" \
    "$script_path"

  run_expect_exit_code \
    "memory log fixture fails when required updater is mismatched" \
    1 \
    "$repo_dir" \
    env \
    MEMORY_REQUIRED_UPDATER=Human \
    "$script_path"

  run_expect_exit_code \
    "memory log fixture passes when required updater matches" \
    0 \
    "$repo_dir" \
    env \
    MEMORY_REQUIRED_UPDATER=Codex \
    "$script_path"
}

run_expect_exit_code \
  "bash -n scripts/run_agent_request.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_agent_request.sh

run_expect_exit_code \
  "bash -n scripts/run_agent_ci_checks.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_agent_ci_checks.sh

run_expect_exit_code \
  "bash -n scripts/run_agent_preflight.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_agent_preflight.sh

run_expect_exit_code \
  "bash -n scripts/install_agent_preflight_hooks.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/install_agent_preflight_hooks.sh

run_expect_exit_code \
  "bash -n scripts/refresh_memory_md.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/refresh_memory_md.sh

run_expect_exit_code \
  "run_agent_request --help" \
  0 \
  "$ROOT_DIR" \
  scripts/run_agent_request.sh \
  --help

run_expect_exit_code \
  "run_agent_preflight --help" \
  0 \
  "$ROOT_DIR" \
  scripts/run_agent_preflight.sh \
  --help

run_expect_exit_code \
  "bash -n scripts/check_file_size_budget.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_file_size_budget.sh

run_expect_exit_code \
  "bash -n scripts/check_rust_taste_invariants.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_rust_taste_invariants.sh

run_expect_exit_code \
  "bash -n scripts/check_run_contract_smoke.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_run_contract_smoke.sh

run_expect_exit_code \
  "bash -n scripts/check_quality_score_drift.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_quality_score_drift.sh

run_expect_exit_code \
  "bash -n scripts/check_rust_private_docs.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_rust_private_docs.sh

run_expect_exit_code \
  "bash -n scripts/run_perf_guard.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_perf_guard.sh

run_expect_exit_code \
  "bash -n scripts/run_perf_wheel_stability.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_perf_wheel_stability.sh

run_expect_exit_code \
  "bash -n scripts/run_sandbox.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_sandbox.sh

run_file_size_budget_fixture
run_taste_invariants_fixture
run_run_contract_smoke_fixture
run_quality_score_drift_fixture
run_memory_log_fixture

if (( failures > 0 )); then
  echo "[guardrails] FAILED: $failures checks failed."
  exit 1
fi

echo "[guardrails] OK"
exit 0
