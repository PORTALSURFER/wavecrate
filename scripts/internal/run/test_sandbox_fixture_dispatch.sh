#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
TEST_ROOT="$(mktemp -d -t wavecrate-fixture-dispatch-XXXXXXXX)"
trap 'rm -rf "$TEST_ROOT"' EXIT

FAKE_BIN="$TEST_ROOT/bin"
CALLS="$TEST_ROOT/cargo-calls.txt"
mkdir -p "$FAKE_BIN"
cat >"$FAKE_BIN/cargo" <<'SH'
#!/usr/bin/env bash
if [[ -n "${WAVECRATE_SOURCE_DB_READ_ONLY:-}" ]]; then
  echo "fixture/app cargo inherited WAVECRATE_SOURCE_DB_READ_ONLY" >&2
  exit 91
fi
printf '%s\n' "$*" >>"$WAVECRATE_FIXTURE_TEST_CALLS"
exit 0
SH
chmod +x "$FAKE_BIN/cargo"

export PATH="$FAKE_BIN:$PATH"
export WAVECRATE_FIXTURE_TEST_CALLS="$CALLS"
export WAVECRATE_SOURCE_DB_READ_ONLY=1

"$ROOT_DIR/scripts/internal/run/run_sandbox.sh" \
  --dir "$TEST_ROOT/sandbox" \
  --fixture small-multi-source \
  --fixture-preserve \
  -- \
  --log >/dev/null

grep -Fq \
  "run --quiet --bin wavecrate-fixture -- provision --fixture small-multi-source --config-base $TEST_ROOT/sandbox --profile sandbox --no-reset" \
  "$CALLS"
grep -Fq "run --release -- --log" "$CALLS"

: >"$CALLS"
export WAVECRATE_SOURCE_DB_READ_ONLY=1
"$ROOT_DIR/scripts/internal/run/run_sandbox.sh" \
  --dir "$TEST_ROOT/reset" \
  --fixture empty >/dev/null
grep -Fxq \
  "run --quiet --bin wavecrate-fixture -- provision --fixture empty --config-base $TEST_ROOT/reset --profile sandbox" \
  "$CALLS"

set +e
invalid_output="$(
  "$ROOT_DIR/scripts/internal/run/run_sandbox.sh" \
    --dir "$TEST_ROOT/invalid" \
    --fixture live 2>&1
)"
invalid_status=$?
set -e
if (( invalid_status != 2 )); then
  echo "expected invalid fixture to exit 2, got $invalid_status" >&2
  exit 1
fi
grep -Fq "Unknown fixture 'live'" <<<"$invalid_output"

echo "sandbox fixture dispatch checks passed"
