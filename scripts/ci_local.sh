#!/usr/bin/env bash

# Local CI mirror: run the same checks as `.github/workflows/ci.yml`.
# This script is intentionally simple so agents and humans can rely on it.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[ci_local] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[ci_local] scripts/check_memory_log.sh"
./scripts/check_memory_log.sh

echo "[ci_local] scripts/check_migration_boundary.sh"
./scripts/check_migration_boundary.sh

echo "[ci_local] scripts/check_script_guardrails.sh"
./scripts/check_script_guardrails.sh

echo "[ci_local] scripts/check_workflow_toolchain_pinning.sh"
./scripts/check_workflow_toolchain_pinning.sh

echo "[ci_local] scripts/check_file_size_budget.sh"
./scripts/check_file_size_budget.sh

echo "[ci_local] scripts/check_manual_docs_scope.sh"
./scripts/check_manual_docs_scope.sh

echo "[ci_local] scripts/check_legacy_app_coupling.sh"
./scripts/check_legacy_app_coupling.sh

echo "[ci_local] scripts/check_rust_taste_invariants.sh"
./scripts/check_rust_taste_invariants.sh

echo "[ci_local] scripts/check_rust_no_todos.sh"
./scripts/check_rust_no_todos.sh

echo "[ci_local] scripts/check_rust_public_docs.sh"
./scripts/check_rust_public_docs.sh

echo "[ci_local] scripts/check_app_core_dependency_boundary.sh"
./scripts/check_app_core_dependency_boundary.sh

echo "[ci_local] scripts/knowledge_lint.sh"
./scripts/knowledge_lint.sh

echo "[ci_local] cargo clippy --all-targets"
cargo clippy --all-targets

echo "[ci_local] cargo doc -p sempal --no-deps (RUSTDOCFLAGS=-D warnings)"
RUSTDOCFLAGS="-D warnings" cargo doc -p sempal --no-deps

echo "[ci_local] cargo test --all-targets"
cargo test --all-targets

echo "[ci_local] OK"
