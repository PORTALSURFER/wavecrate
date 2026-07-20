#!/usr/bin/env bash

# Keeps readiness execution owned by the normal library graph and prevents the
# retired legacy-controller bridge from returning.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

failures=0

fail() {
  echo "[readiness_executor_boundary] $1" >&2
  failures=$((failures + 1))
}

require_literal() {
  local path="$1"
  local literal="$2"
  local message="$3"
  if ! grep -Fq -- "$literal" "$path"; then
    fail "$message"
  fi
}

if [[ -e src/internal_analysis_jobs.rs ]]; then
  fail "src/internal_analysis_jobs.rs is a retired bridge and must not exist."
fi

if find src/app/controller/library/analysis_jobs/pool/job_execution \
  -type f -name '*.rs' -print -quit 2>/dev/null | grep -q .; then
  fail "Readiness executors must not live under the legacy controller tree."
fi

if [[ ! -f src/readiness_execution/mod.rs ]]; then
  fail "Missing library-owned src/readiness_execution/mod.rs."
else
  if grep -REn --include='*.rs' \
    'crate::(app|app_core)::|internal_analysis_jobs|app::controller::library::analysis_jobs' \
    src/readiness_execution >/dev/null; then
    fail "src/readiness_execution must not depend on legacy app/controller modules."
  fi
fi

require_literal Cargo.toml 'default = []' \
  "The default Wavecrate feature set must remain free of legacy-controller."
require_literal src/lib.rs 'pub mod readiness_execution;' \
  "The readiness executor must remain an explicit library API."
require_literal tools/gui-test-cli/Cargo.toml 'features = ["legacy-controller"]' \
  "gui-test-cli must opt into legacy-controller explicitly."
require_literal tools/bench-cli/Cargo.toml 'features = ["legacy-controller"]' \
  "wavecrate-bench-cli must opt into legacy-controller explicitly."

for declaration in 'mod app;' 'pub mod app_core;' 'pub mod gui_test;'; do
  if ! grep -F -B2 -- "$declaration" src/lib.rs \
    | grep -Fq '#[cfg(any(test, feature = "legacy-controller"))]'; then
    fail "$declaration must remain gated by test or legacy-controller."
  fi
done

if grep -REn --include='*.rs' \
  'internal_analysis_jobs|analysis_jobs::(run_feature_stage|run_embedding_stage)' \
  src/native_app/source_processing >/dev/null; then
  fail "Native source processing must call the readiness-owned executor API directly."
fi

if (( failures > 0 )); then
  exit 1
fi

echo "[readiness_executor_boundary] OK"
