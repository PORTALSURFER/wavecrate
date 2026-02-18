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
  "bash -n scripts/check_quality_score_drift.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/check_quality_score_drift.sh

run_expect_exit_code \
  "bash -n scripts/run_sandbox.sh" \
  0 \
  "$ROOT_DIR" \
  bash \
  -n \
  scripts/run_sandbox.sh

run_file_size_budget_fixture
run_taste_invariants_fixture
run_quality_score_drift_fixture

if (( failures > 0 )); then
  echo "[guardrails] FAILED: $failures checks failed."
  exit 1
fi

echo "[guardrails] OK"
exit 0
