# Development Workflow and Test Map

This document is the canonical developer workflow guide for local iteration,
validation, and test selection.

## Default validation ladder

Use the lightest lane that still gives trustworthy coverage for the change.

1. Tight edit loop
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 smoke`
   - macOS/Linux/WSL:
     `bash scripts/ci.sh smoke`
2. Agent-safe validation
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
   - macOS/Linux/WSL:
     `bash scripts/ci.sh agent`
3. Broader integrated local checks
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
   - macOS/Linux/WSL:
     `bash scripts/ci.sh quick`
4. Full local CI parity
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 local`
   - macOS/Linux/WSL:
     `bash scripts/ci.sh local`

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

- `bash scripts/check.sh golden-tests`
- `cargo nextest run golden_log_mel_matches_python`
- `cargo nextest run golden_embedding_matches_python`

### Sempal native-shell tests

Use for compatibility-shell visual and behavior changes that are Sempal-owned.

- targeted snapshot suites:
  - `startup_shot_matches_fixture`
  - `browser_dense_shot_matches_fixture`
  - `waveform_selection_shot_matches_fixture`
  - update fixtures with
    `cargo test -p sempal --lib update_shot_fixtures -- --ignored`

### Radiant native-shell tests

Use for generic Radiant behavior and explicit compatibility coverage while
`compat::sempal_shell` exists.

- `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`

### GUI test platform

Use for semantic GUI contracts, CLI scenarios, and desktop AIV loops.

- Automated GUI runs default to the isolated `isolated-startup` fixture, which
  exercises persisted startup against the dedicated `automated-tests`
  persistence profile.
- Use the `live` GUI fixture only for deliberate manual validation against the
  real persisted startup profile.
- Treat `default` only as a legacy alias for `isolated-startup`; it is not a
  separate live-profile mode.

- Browser preview/commit flicker regressions:
  - targeted controller checks: `cargo test browser_focus_transition`
  - semantic GUI proof lane: `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`
  - contract scenario anchor: `browser_focus_transition_stability`
- contract loop:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`
- broader GUI suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 suite`
- live AIV smoke:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 aiv-smoke`
- live AIV suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 aiv-suite -PackName desktop-regression`

See `docs/SYSTEMS.md` for the GUI artifact and automation contract details.

### Persistence profile boundary

Use these checks when the change could affect startup config loading/saving,
sample-source persistence, or GUI fixture profile selection.

- profile model:
  - live runs use `<config-base>/.sempal/`
  - sandbox/manual QA runs use `<config-base>/.sempal/profiles/sandbox/`
  - automated validation runs use `<config-base>/.sempal/profiles/automated-tests/`
- regression anchors:
  - `cargo test app_core::controller::tests::persistence_boundary::`
  - `cargo test gui_test::`
- wrapper coverage:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`
    now includes the persistence-boundary regression that snapshots the live
    `library.db` bytes before and after an isolated controller run
  - `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
    still covers the same regression through `cargo test -p sempal --lib`

### Windows manual confirmation

Use this after boundary/profile work when you want one explicit PowerShell proof
that validation did not leak fixture sources into the real startup profile.

1. Run `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`.
2. Run `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`.
3. Start one normal live runtime without `SEMPAL_CONFIG_HOME` or
   `SEMPAL_CONFIG_PROFILE` overrides:
   `cargo run --release`
4. Confirm the app opens with the intended real source list instead of temp or
   fixture paths.
5. If you need a sandbox/manual QA run instead, use
   `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 sandbox --`
   so the session writes to the dedicated `sandbox` profile instead of the live
   profile.

### Debug log workflow

Use this when reproducing a release/manual bug that needs the richer
per-launch diagnostics trail.

1. Start a repro run with the Sempal-owned debug flag:
   `cargo run --release -- --log`
2. For isolated manual QA, use the sandbox profile instead:
   `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 sandbox -- --log`
3. Retrieve the newest log quickly:
   `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs`
4. Retrieve the sandbox run log the same way:
   `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs -Sandbox`
5. Remember the boundary:
   the log captures startup metadata plus structured action/DB diagnostics for
   the current reproduced run only; earlier sessions still require a fresh repro
   with `--log` enabled.
6. Extra-verbose hot-path traces are separate:
   default `--log` keeps routine loop-cycle and similar hot-path chatter out of
   the file; only enable `SEMPAL_HOTPATH_TELEMETRY=1` when you deliberately need
   developer-level loop/hot-path tracing during a focused repro.

### Benchmarks and perf checks

- `cargo bench`
- `cargo bench --bench ann_index`
- `cargo bench --bench tagging`
- local perf guard:
  - `bash scripts/perf.sh guard`
  - `powershell -ExecutionPolicy Bypass -File scripts/perf.ps1 guard`

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
