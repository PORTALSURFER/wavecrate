# Development Workflow and Test Map

This document is the canonical developer workflow guide for local iteration,
validation, and test selection.

## Default validation ladder

Use the lightest lane that still gives trustworthy coverage for the change.

1. Tight edit loop
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
   - macOS/Linux/WSL:
     `bash scripts/devcheck.sh`
2. Agent-safe validation
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
   - macOS/Linux/WSL:
     `bash scripts/ci_agent.sh`
3. Broader integrated local checks
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
   - macOS/Linux/WSL:
     `bash scripts/ci_quick.sh`
4. Full local CI parity
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
   - macOS/Linux/WSL:
     `bash scripts/ci_local.sh`

Windows note:

- use the PowerShell wrappers in this repository
- the wrappers fall back to direct `rustc` plus repo-local temp space when
  inherited `sccache` or the default temp directory is unusable
- do not run multiple cargo test commands concurrently

## Safe feature-change checklist

Before push:

1. Route the change to the right owner using `docs/ARCHITECTURE.md`.
2. Add or update tests for non-trivial logic.
3. Update the canonical doc that owns the changed behavior.
4. Run the appropriate validation lane until green.
5. Keep the diff focused and avoid broad incidental cleanup.

## Test suite map

### Root crate tests

Use for most app/domain behavior under `src/`.

- all project tests:
  - `cargo nextest run --all-targets --no-fail-fast`
  - `cargo test --doc`
- quick app-development subset:
  - `cargo nextest run --profile quick --lib --tests`
- agent-safe library suite:
  - `cargo test -p sempal --lib`

### Script and golden checks

Use for tooling, fixtures, and numerical-reference flows.

- `bash scripts/check/ci_golden_tests.sh`
- `cargo nextest run golden_log_mel_matches_python`
- `cargo nextest run golden_embedding_matches_python`

### Radiant native-shell tests

Use for compatibility-shell visual and behavior changes.

- `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`
- targeted snapshot suites:
  - `startup_shot_matches_fixture`
  - `browser_dense_shot_matches_fixture`
  - `waveform_selection_shot_matches_fixture`

### GUI test platform

Use for semantic GUI contracts, CLI scenarios, and desktop AIV loops.

- contract loop:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui/run_gui_contract.ps1`
- broader GUI suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui/run_gui_suite.ps1`
- live AIV smoke:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui/run_gui_aiv_smoke.ps1`
- live AIV suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui/run_gui_aiv_suite.ps1 -PackName desktop-regression`

See `docs/SYSTEMS.md` for the GUI artifact and automation contract details.

### Benchmarks and perf checks

- `cargo bench`
- `cargo bench --bench ann_index`
- `cargo bench --bench tagging`
- local perf guard:
  - `bash scripts/perf/run_perf_guard.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/perf/run_perf_guard.ps1`

## CI reference

GitHub CI and local wrappers together cover:

- formatting
- clippy
- nextest
- doc tests
- selected guardrail scripts
- local-only perf and GUI contract lanes where appropriate

When in doubt, run the wrapper script instead of assembling the command list by
hand.
