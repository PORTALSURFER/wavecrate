#!/usr/bin/env bash

# Local CI mirror: run the same checks as `.github/workflows/ci.yml`.
# This script is intentionally simple so agents and humans can rely on it.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[ci_local] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[ci_local] scripts/check_migration_boundary.sh"
./scripts/check_migration_boundary.sh

echo "[ci_local] scripts/check_file_size_budget.sh"
./scripts/check_file_size_budget.sh

echo "[ci_local] cargo clippy --all-targets"
cargo clippy --all-targets

echo "[ci_local] cargo test --all-targets"
cargo test --all-targets

echo "[ci_local] OK"
