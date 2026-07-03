#!/usr/bin/env bash
set -euo pipefail

echo "[release_validation] Build Wavecrate workspace test targets."
cargo test --workspace --locked --exclude radiant --no-run

echo "[release_validation] Run release workflow contract checks."
cargo test --test release_contract
cargo test --test release_workflow_helpers

echo "[release_validation] Run scanner tests used by release-time source validation."
cargo test -p wavecrate-scan --lib

echo "[release_validation] OK"
