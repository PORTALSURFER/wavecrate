# Test Suite Map

This file inventories the test suites currently exercised in the repository and the preferred commands to run them.

## Fast Development Loop

Use these for normal iteration:

- Lightest app-only compile gate:
  - `bash scripts/devcheck_app.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck_app.ps1`
- Fastest smoke/compile gate:
  - `bash scripts/devcheck.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Agent-safe local validation checks:
  - `bash scripts/ci_agent.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - On Windows, the PowerShell wrappers probe inherited `sccache` wrappers and fall back to direct `rustc` plus `tmp/agent_temp` when the wrapper or default temp dir is not usable.
- Broader integrated local development checks:
  - `bash scripts/ci_quick.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Full CI parity checks:
  - `bash scripts/ci_local.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

Recommended cadence:
- Use `devcheck_app` when you are only changing the main app/runtime path and
  want the shortest compile loop.
- Use `devcheck` during the tight edit loop.
- Use `ci_agent` when you need a reliable agent-safe gate in constrained environments where `cargo-nextest` or spawned test executables are blocked.
- Use `ci_quick` for the broader integrated local lane before commit when `cargo nextest` is available; on Windows the PowerShell wrapper also adds the GUI contract wrapper.
- Use `ci_local` for tooling changes, dependency work, perf-sensitive work, or when you need CI parity.

## 1) Root crate unit + integration tests (`sempal`)

Location: `src/` modules with `#[cfg(test)]` blocks and `tests/` integration files.

- Run all project tests:
  - `cargo nextest run --all-targets --no-fail-fast`
  - `cargo test --doc`
- Run the filtered quick app-development subset:
  - `cargo nextest run --profile quick --lib --tests`
- Run the agent-safe library suite in one cargo process:
  - `cargo test -p sempal --lib -- --test-threads=1`
- Run only integration tests:
  - `cargo nextest run --test controller_browser_integration`
  - `cargo nextest run --test take_duration_test`
  - `cargo nextest run --test repro_duration`
- Focus a specific integration harness:
  - `cargo nextest run --test controller_browser_integration click_clears_selection_and_focuses_row`

## 2) Golden/pandas-style regression checks (`scripts/`)

Location: golden reference scripts that validate ANN/PANN numerical outputs and CI wiring.

- Generate/update golden references:
  - `bash scripts/ci_golden_tests.sh`
  - `python3 tools/generate_panns_golden_mel.py --out assets/ml/panns_cnn14_16k/golden_mel.json` (if available)
  - `python3 tools/generate_panns_golden_embedding.py --out assets/ml/panns_cnn14_16k/golden_embedding.json` (if available)
- Run golden regression tests:
  - `cargo nextest run golden_log_mel_matches_python`
  - `cargo nextest run golden_embedding_matches_python`

## 3) Vendor native UI crate visual/behavior regression (`vendor/radiant`)

Location: `vendor/radiant/src/gui/native_shell` unit tests and `vendor/radiant/tests/shots` fixtures.

- Run unit tests for native-shell logic:
  - `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`
- Run snapshot compare suites:
  - `cargo nextest run --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture`
  - `cargo nextest run --manifest-path vendor/radiant/Cargo.toml browser_dense_shot_matches_fixture`
  - `cargo nextest run --manifest-path vendor/radiant/Cargo.toml waveform_selection_shot_matches_fixture`
- Regenerate fixture baselines (on intentional UI/layout changes):
  - `cargo nextest run --manifest-path vendor/radiant/Cargo.toml native_shell::shots::update_shot_fixtures --run-ignored only`

## 4) GUI test platform loop

Location:

- catalog + runner contracts: `src/app_core/actions`, `src/gui_test`
- native-shell automation snapshot: `vendor/radiant/src/gui/native_shell/state/automation.rs`
- CLI: `tools/gui-test-cli`

Commands:

- Fast semantic/runtime GUI contract loop:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
- Broader GUI suite loop:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`
- Live semantic AIV smoke wrapper:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_smoke.ps1`
- Live semantic AIV desktop suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_suite.ps1 -PackName desktop-regression`
- Direct CLI snapshot export:
  - `cargo run -p gui-test-cli -- snapshot artifacts/gui-test/gui-test-snapshot.json`
- Direct CLI action dispatch:
  - `cargo run -p gui-test-cli -- dispatch-action "\"ToggleTransport\"" artifacts/gui-test/dispatch.json`
- Direct CLI scenario run:
  - `cargo run -p gui-test-cli -- run-scenario artifacts/gui-test/scenario.json artifacts/gui-test/scenario-report.json`
- Direct CLI scenario-pack run:
  - `cargo run -p gui-test-cli -- run-scenario-pack contract-smoke artifacts/gui-test/scenario-pack`
- Export the desktop-AIV smoke pack manifest:
  - `cargo run -p gui-test-cli -- export-aiv-suite artifacts/gui-aiv/suite-manifest.json`
- Export a named desktop-AIV manifest:
  - `cargo run -p gui-test-cli -- export-aiv-suite desktop-regression artifacts/gui-aiv/suite-manifest.json`
- Resolve one semantic node target from a live artifact:
  - `cargo run -p gui-test-cli -- resolve-node-target artifacts/gui-test/gui-test-snapshot.json shell.top_bar.options_button`

Current named fixture tags:

- `default`
- `browser`
- `sources`
- `waveform`
- `options`
- `prompt`
- `update`

## 5) Benchmarks

Location: `[[bench]]` targets in `Cargo.toml`.

- Run all benchmarks:
  - `cargo bench`
- Run a specific benchmark:
  - `cargo bench --bench ann_index`
  - `cargo bench --bench tagging`

## 6) Manual and support checks

- Migration boundary enforcement:
  - `./scripts/check_migration_boundary.sh`
- Formatting and lint baseline (also run in CI):
  - `cargo fmt --all`
  - `cargo clippy --all-targets`

## 7) CI matrix (reference)

See `.github/workflows/ci.yml` for branch-wide runs:
- Runs on `main` and `next`.
- Executes `cargo fmt --all -- --check`, `cargo clippy --all-targets`, `cargo nextest run --all-targets --no-fail-fast`, and `cargo test --doc`.
