# Test Suite Map

This file inventories the test suites currently exercised in the repository and the preferred commands to run them.

## 1) Root crate unit + integration tests (`sempal`)

Location: `src/` modules with `#[cfg(test)]` blocks and `tests/` integration files.

- Run all project tests:
  - `cargo test --all-targets`
- Run only integration tests:
  - `cargo test --test controller_browser_integration`
  - `cargo test --test take_duration_test`
  - `cargo test --test repro_duration`
- Focus a specific integration harness:
  - `cargo test --test controller_browser_integration click_clears_selection_and_focuses_row`

## 2) Golden/pandas-style regression checks (`scripts/`)

Location: golden reference scripts that validate ANN/PANN numerical outputs and CI wiring.

- Generate/update golden references:
  - `bash scripts/ci_golden_tests.sh`
  - `python3 tools/generate_panns_golden_mel.py --out assets/ml/panns_cnn14_16k/golden_mel.json` (if available)
  - `python3 tools/generate_panns_golden_embedding.py --out assets/ml/panns_cnn14_16k/golden_embedding.json` (if available)
- Run golden regression tests:
  - `cargo test golden_log_mel_matches_python`
  - `cargo test golden_embedding_matches_python`

## 3) Vendor native UI crate visual/behavior regression (`vendor/radiant`)

Location: `vendor/radiant/src/gui/native_shell` unit tests and `vendor/radiant/tests/shots` fixtures.

- Run unit tests for native-shell logic:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml`
- Run snapshot compare suites:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml browser_dense_shot_matches_fixture`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml waveform_selection_shot_matches_fixture`
- Regenerate fixture baselines (on intentional UI/layout changes):
  - `cargo test --manifest-path vendor/radiant/Cargo.toml native_shell::shots::update_shot_fixtures -- --ignored`

## 4) Benchmarks

Location: `[[bench]]` targets in `Cargo.toml`.

- Run all benchmarks:
  - `cargo bench`
- Run a specific benchmark:
  - `cargo bench --bench ann_index`
  - `cargo bench --bench tagging`

## 5) Manual and support checks

- Migration boundary enforcement:
  - `./scripts/check_migration_boundary.sh`
- Formatting and lint baseline (also run in CI):
  - `cargo fmt --all`
  - `cargo clippy --all-targets`

## 6) CI matrix (reference)

See `.github/workflows/ci.yml` for branch-wide runs:
- Runs on `main` and `next`.
- Executes `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and `cargo test --all-targets`.
