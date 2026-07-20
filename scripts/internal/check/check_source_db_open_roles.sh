#!/usr/bin/env bash

# Reject ambiguous SourceDatabase opens so every caller declares its runtime role.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT_DIR"

call_pattern='SourceDatabase::(open|open_with_database_root|open_fast|open_fast_with_database_root|open_read_only|open_read_only_with_database_root|open_connection)[[:space:]]*\('
declaration_pattern='pub fn (open|open_with_database_root|open_fast|open_fast_with_database_root|open_read_only|open_read_only_with_database_root|open_connection)[[:space:]]*\('
scope=(benches crates src tests tools)
violations=""

if command -v rg >/dev/null 2>&1; then
  violations="$(
    {
      rg -n --glob '*.rs' "$call_pattern" "${scope[@]}" || true
      rg -n "$declaration_pattern" crates/wavecrate-library/src/sample_sources/db/mod.rs || true
    } | sort -u
  )"
else
  violations="$(
    {
      grep -REn --include='*.rs' "$call_pattern" "${scope[@]}" || true
      grep -En "$declaration_pattern" crates/wavecrate-library/src/sample_sources/db/mod.rs || true
    } | sort -u
  )"
fi

if [[ -n "$violations" ]]; then
  echo "[source_db_roles] Ambiguous SourceDatabase opens are forbidden:" >&2
  echo "$violations" >&2
  echo "[source_db_roles] Use an explicit role-specific API; fixture setup may use open_for_test_fixture_source_write." >&2
  exit 1
fi

echo "[source_db_roles] OK"
