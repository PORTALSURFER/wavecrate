#!/usr/bin/env bash

# Guardrail checks for scripts used by the agent harness.
#
# This script verifies:
# - shell syntax checks for key scripts
# - fixture-based checks for `.rs` matching in file-size budget logic
# - fixture-based checks for diff-aware matching in taste invariants

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
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

assert_file_contains() {
  local label="$1"
  local path="$2"
  local fragment="$3"

  if grep -Fq -- "$fragment" "$path"; then
    echo "[guardrails] PASS: $label"
  else
    echo "[guardrails] FAIL: $label" >&2
    echo "[guardrails] Missing fragment '$fragment' in $path" >&2
    failures=$((failures + 1))
  fi
}

run_cleanup_audit_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/internal/check/audit_cleanup_hotspots.sh"
  local output_path="$repo_dir/tmp/cleanup.md"
  mkdir -p "$repo_dir/src/analysis" "$repo_dir/src/selection" "$repo_dir/vendor/radiant/src" "$repo_dir/scripts/internal/check" "$repo_dir/tmp"
  cp "scripts/internal/check/audit_cleanup_hotspots.sh" "$script_path"
  chmod +x "$script_path"

  cat >"$repo_dir/src/analysis/ann_index_tests.rs" <<'EOF'
fn alpha() {}
fn beta() {}
fn gamma() {}
EOF
  cat >"$repo_dir/src/selection/mod.rs" <<'EOF'
#[cfg(test)]
mod tests;
EOF
  cat >"$repo_dir/src/selection/tests.rs" <<'EOF'
#[test]
fn selection_is_covered() {}
EOF
  cat >"$repo_dir/src/selection/range.rs" <<'EOF'
pub fn start() {}
pub fn end() {}
pub fn clamp() {}
EOF
  cat >"$repo_dir/src/real_gap.rs" <<'EOF'
pub fn one() {}
pub fn two() {}
pub fn three() {}
EOF
  cat >"$repo_dir/vendor/radiant/src/vendor_gap.rs" <<'EOF'
pub fn vendor_one() {}
pub fn vendor_two() {}
pub fn vendor_three() {}
EOF

  git -C "$repo_dir/vendor/radiant" init -q
  git -C "$repo_dir/vendor/radiant" config user.name "sempal-ci"
  git -C "$repo_dir/vendor/radiant" config user.email "ci@sempal.test"
  git -C "$repo_dir/vendor/radiant" add .
  git -C "$repo_dir/vendor/radiant" commit -qm "seed"

  git -C "$repo_dir" init -q
  git -C "$repo_dir" config user.name "sempal-ci"
  git -C "$repo_dir" config user.email "ci@sempal.test"
  git -C "$repo_dir" add src scripts
  git -C "$repo_dir" commit -qm "seed"

  run_expect_exit_code \
    "cleanup-audit fixture succeeds" \
    0 \
    "$repo_dir" \
    "$script_path" \
    --output \
    "$output_path" \
    --test-gap-min-lines \
    3 \
    --top-files \
    10

  root_gap_section="$(sed -n '/^## Sempal-root likely test-gap hotspots (heuristic)/,/^## Vendor\/Radiant likely test-gap hotspots (heuristic)/p' "$output_path")"
  vendor_gap_section="$(sed -n '/^## Vendor\/Radiant likely test-gap hotspots (heuristic)/,/^## Suggested follow-up/p' "$output_path")"

  if grep -Fq 'Likely large-file test-gap hotspots (heuristic): 2' "$output_path"; then
    echo "[guardrails] PASS: cleanup-audit fixture reports two scoped heuristic gaps"
  else
    echo "[guardrails] FAIL: cleanup-audit fixture reports unexpected heuristic gap count" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  fi

  if grep -Fq '## Sempal-root largest Rust files' "$output_path" && grep -Fq '## Vendor/Radiant largest Rust files' "$output_path"; then
    echo "[guardrails] PASS: cleanup-audit fixture emits root/vendor sections"
  else
    echo "[guardrails] FAIL: cleanup-audit fixture did not emit root/vendor sections" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  fi

  if grep -Fq '`src/real_gap.rs`' <<<"$root_gap_section"; then
    echo "[guardrails] PASS: cleanup-audit fixture keeps the real gap"
  else
    echo "[guardrails] FAIL: cleanup-audit fixture missed the real gap" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  fi

  if grep -Fq '`vendor/radiant/src/vendor_gap.rs`' <<<"$vendor_gap_section" && ! grep -Fq '`vendor/radiant/src/vendor_gap.rs`' <<<"$root_gap_section"; then
    echo "[guardrails] PASS: cleanup-audit fixture keeps the vendor gap separate"
  else
    echo "[guardrails] FAIL: cleanup-audit fixture did not keep the vendor gap separate" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  fi

  if grep -Fq '`src/analysis/ann_index_tests.rs`' <<<"$root_gap_section"; then
    echo "[guardrails] FAIL: cleanup-audit fixture still flags *_tests.rs files" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  else
    echo "[guardrails] PASS: cleanup-audit fixture skips *_tests.rs files"
  fi

  if grep -Fq '`src/selection/range.rs`' <<<"$root_gap_section"; then
    echo "[guardrails] FAIL: cleanup-audit fixture still flags sibling module tests" >&2
    cat "$output_path" >&2
    failures=$((failures + 1))
  else
    echo "[guardrails] PASS: cleanup-audit fixture skips sibling module tests"
  fi
}

run_docs_index_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/internal/check/check_docs_index.sh"
  mkdir -p "$repo_dir/scripts/internal/check"
  cp "scripts/internal/check/check_docs_index.sh" "$script_path"
  chmod +x "$script_path"

  for path in \
    docs/ARCHITECTURE.md \
    docs/ENV_VARS.md \
    docs/TEST.md \
    docs/SYSTEMS.md \
    docs/TROUBLESHOOTING.md
  do
    mkdir -p "$repo_dir/$(dirname "$path")"
    : >"$repo_dir/$path"
  done

  cat >"$repo_dir/docs/README.md" <<'EOF'
# Developer documentation

- `docs/ARCHITECTURE.md`
- `docs/ENV_VARS.md`
- `docs/TEST.md`
- `docs/SYSTEMS.md`
- `docs/TROUBLESHOOTING.md`
- `AGENTS.md`
- Planning and backlog
  - live in Linear project `Sempal` under team `PORTALSURFER`
EOF

  run_expect_exit_code \
    "docs-index fixture accepts Linear planning pointer" \
    0 \
    "$repo_dir" \
    "$script_path"

  cat >"$repo_dir/docs/README.md" <<'EOF'
# Developer documentation

- `docs/ARCHITECTURE.md`
- `docs/ENV_VARS.md`
- `docs/TEST.md`
- `docs/SYSTEMS.md`
- `docs/TROUBLESHOOTING.md`
- `AGENTS.md`
- `docs/plans/index.md`
EOF

  run_expect_exit_code \
    "docs-index fixture rejects legacy markdown planning entrypoints" \
    1 \
    "$repo_dir" \
    "$script_path"
}

run_file_size_budget_fixture() {
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  trap 'rm -rf "$fixture_dir"' RETURN

  local repo_dir="$fixture_dir/repo"
  local script_path="$repo_dir/scripts/internal/check/check_file_size_budget.sh"
  mkdir -p "$repo_dir/src" "$repo_dir/scripts/internal/check" "$repo_dir/vendor"
  cp "scripts/internal/check/check_file_size_budget.sh" "$script_path"
  chmod +x "$script_path"

  git -C "$fixture_dir/repo" init -q
  git -C "$fixture_dir/repo" config user.name "sempal-ci"
  git -C "$fixture_dir/repo" config user.email "ci@sempal.test"

  mkdir -p "$fixture_dir/repo/tests"
  cat >"$fixture_dir/repo/src/too_many_lines.rs" <<'EOF'
fn ok() {
    println!("budget");
}
EOF
  cat >"$fixture_dir/repo/tests/notes.txt" <<'EOF'
plain text should not be checked by script
EOF

  git -C "$fixture_dir/repo" add src/too_many_lines.rs
  git -C "$fixture_dir/repo" add tests/notes.txt
  git -C "$fixture_dir/repo" commit -qm "seed"

  mkdir -p "$fixture_dir/repo/vendor/radiant/src"
  git -C "$fixture_dir/repo/vendor/radiant" init -q
  git -C "$fixture_dir/repo/vendor/radiant" config user.name "sempal-ci"
  git -C "$fixture_dir/repo/vendor/radiant" config user.email "ci@sempal.test"
  cat >"$fixture_dir/repo/vendor/radiant/src/too_many_lines.rs" <<'EOF'
fn main() {
    println!("a");
    println!("b");
    println!("c");
    println!("d");
}
EOF
  git -C "$fixture_dir/repo/vendor/radiant" add src/too_many_lines.rs
  git -C "$fixture_dir/repo/vendor/radiant" commit -qm "seed"

  run_expect_exit_code \
    "file-size-budget fixture catches over-limit nested vendor file" \
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
  local script_path="$repo_dir/scripts/internal/check/check_rust_taste_invariants.sh"
  mkdir -p "$repo_dir/src" "$repo_dir/scripts/internal/check"
  cp "scripts/internal/check/check_rust_taste_invariants.sh" "$script_path"
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

  local fixture_script="$ROOT_DIR/scripts/internal/check/check_run_contract_smoke.sh"
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

run_expect_exit_code \
  "bash -n scripts/agent.sh request" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/agent.sh request

run_expect_exit_code \
  "bash -n scripts/internal/agent/run_agent_ci_checks.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/agent/run_agent_ci_checks.sh

run_expect_exit_code \
  "bash -n scripts/internal/agent/run_agent_preflight.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/agent/run_agent_preflight.sh

run_expect_exit_code \
  "bash -n scripts/ci.sh smoke" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/ci.sh smoke

run_expect_exit_code \
  "bash -n scripts/ci.sh agent" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/ci.sh agent

run_expect_exit_code \
  "bash -n scripts/ci.sh quick" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/ci.sh quick

run_expect_exit_code \
  "bash -n scripts/internal/agent/install_agent_preflight_hooks.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/agent/install_agent_preflight_hooks.sh

run_expect_exit_code \
  "run_agent_request --help" \
  0 \
  "$ROOT_DIR" \
  scripts/agent.sh request \
  --help

run_expect_exit_code \
  "devcheck --help" \
  0 \
  "$ROOT_DIR" \
  scripts/ci.sh smoke \
  --help

assert_file_contains \
  "PowerShell devcheck checks Radiant standalone example" \
  "scripts/internal/ci/devcheck.ps1" \
  "--example generic_native --no-default-features"

assert_file_contains \
  "Bash devcheck checks Radiant standalone example" \
  "scripts/internal/ci/devcheck.sh" \
  "--example generic_native --no-default-features"

run_expect_exit_code \
  "ci_agent --help" \
  0 \
  "$ROOT_DIR" \
  scripts/ci.sh agent \
  --help

run_expect_exit_code \
  "ci_quick --help" \
  0 \
  "$ROOT_DIR" \
  scripts/ci.sh quick \
  --help

run_expect_exit_code \
  "run_agent_preflight --help" \
  0 \
  "$ROOT_DIR" \
  scripts/internal/agent/run_agent_preflight.sh \
  --help

run_expect_exit_code \
  "bash -n scripts/internal/check/check_file_size_budget.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_file_size_budget.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/check_docs_index.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_docs_index.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/audit_cleanup_hotspots.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/audit_cleanup_hotspots.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/check_rust_taste_invariants.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_rust_taste_invariants.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/check_run_contract_smoke.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_run_contract_smoke.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/check_rust_private_docs.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_rust_private_docs.sh

run_expect_exit_code \
  "bash -n scripts/internal/check/check_rust_dead_deps_advisory.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/check/check_rust_dead_deps_advisory.sh

run_expect_exit_code \
  "bash -n scripts/internal/perf/run_perf_guard.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/perf/run_perf_guard.sh

run_expect_exit_code \
  "bash -n scripts/internal/perf/calibrate_startup_thresholds.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/perf/calibrate_startup_thresholds.sh

run_expect_exit_code \
  "bash -n scripts/internal/perf/run_perf_wheel_stability.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/internal/perf/run_perf_wheel_stability.sh

run_expect_exit_code \
  "bash -n scripts/run.sh sandbox" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run.sh sandbox

run_file_size_budget_fixture
run_cleanup_audit_fixture
run_docs_index_fixture
run_taste_invariants_fixture
run_run_contract_smoke_fixture

if (( failures > 0 )); then
  echo "[guardrails] FAILED: $failures checks failed."
  exit 1
fi

echo "[guardrails] OK"
exit 0
