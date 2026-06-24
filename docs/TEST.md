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
   - load-sensitive selection-export background-job tests run serially here
     after the default library test pass.
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

## CI lane contract

GitHub CI and local wrappers share this contract:

| Lane | GitHub job | Local command | Blocking policy |
| --- | --- | --- | --- |
| Required format and guardrails | `Required: format and guardrails` | `scripts/ci.* local` includes the same required guardrails; use `scripts/check.* <name>` for focused reruns | Required for PRs and pushes to `main` |
| Required clippy | `Required: clippy` | `scripts/ci.* local` | Required for PRs and pushes to `main` |
| Required docs | `Required: docs` | `scripts/ci.* local` | Required for PRs and pushes to `main` |
| Required tests | `Required: tests (linux/windows/macos)` | `scripts/ci.* local` runs `cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast` on the local platform | Required for PRs and pushes to `main` |
| Quarantined/slow tests | `Advisory: quarantined tests (linux/windows/macos)` | `cargo nextest run --workspace --profile ci-quarantine --all-targets --no-fail-fast` | Advisory until the tests are hardened |
| Dead dependency sweep and env-var nudge | `Advisory: repository hygiene` | `scripts/check.* dead-deps --advisory` and `scripts/check.* report-env-vars` | Advisory Linux-only hygiene |
| GUI semantic contracts | none in required GitHub CI | `scripts/gui.ps1 contract` or `scripts/gui.ps1 suite` | Local/manual or issue-specific |
| Perf guard | none in required GitHub CI | `scripts/perf.* guard` | Local/manual or release-risk validation |
| Release build/sync | `Create release on main` after successful `CI`; scheduled `nightly` from `next` only when `next` is ahead of the `nightly` tag | release workflow dispatch | Release-only, after required CI is green; nightly-only for rolling `next` builds |

The nextest policy is:

- `ci-required` is the merge-blocking test subset and excludes known timing,
  platform, or permission-sensitive tests while they are being hardened.
- `ci-quarantine` owns that excluded coverage. Run it deliberately when working
  on source-DB contention, browser async controller integration, or updater
  stale-removal behavior.
- The default nextest profile stays unfiltered for maintainers who want the
  complete suite and are prepared to triage environment-specific failures.
- Required GitHub test jobs upload nextest JUnit and archive summaries on
  failure; start with the job summary before reading the raw log.

The non-blocking app architecture is enforced by
`scripts/check.* non-blocking-architecture`, and that check is required by
`scripts/ci.* agent` plus the agent preflight inside `scripts/ci.* local`. It
runs Radiant's reusable non-blocking guardrails, Wavecrate's app-facing blocking
scan, and the deterministic strict slow-handler diagnostics harness.
The agent lane intentionally runs the focused Radiant non-blocking guardrail
module instead of the full `generic_surface_guardrails` suite, because that
broader suite also contains unrelated cleanup/source-shape guardrails that
should be fixed deliberately rather than used as the non-blocking architecture
gate.

Windows note:

- use the PowerShell wrappers in this repository
- the wrappers use direct `rustc` by default and still fall back to repo-local
  temp space when the default temp directory is unusable
- set `WAVECRATE_ENABLE_SCCACHE=1` only when you explicitly want wrapper caching
- do not run multiple cargo test commands concurrently

## Safe feature-change checklist

Before push:

1. Route the change to the right owner using `docs/TARGET.md`.
2. Add or update tests for non-trivial logic.
3. Update the canonical doc that owns the changed behavior.
4. Run the appropriate validation lane until green.
5. Keep the diff focused and avoid broad incidental cleanup.

## Test suite map

### Root crate tests

Use for most app/domain behavior under `src/`.

- all project tests:
  - `cargo nextest run --all-targets --no-fail-fast`
  - required CI subset:
    `cargo nextest run --workspace --profile ci-required --all-targets --no-fail-fast`
  - quarantined/slow subset:
    `cargo nextest run --workspace --profile ci-quarantine --all-targets --no-fail-fast`
  - `cargo test --doc`
- quick app-development subset:
  - `cargo nextest run --profile quick --lib --tests`
- agent-safe library suite:
  - `cargo test -p wavecrate --lib`
  - `scripts/ci.* agent` runs the library suite with two known legacy
    controller failures skipped so the required agent lane can stay focused on
    deterministic architecture, compile, and broad non-ignored library coverage:
    `prepare_auto_rename_requests_logs_looped_provenance` and
    `rating_previous_random_history_entry_restores_waveform_for_replacement`.

### Script and golden checks

Use for tooling, fixtures, and numerical-reference flows.

- `bash scripts/check.sh golden-tests`
- `cargo nextest run golden_log_mel_matches_python`
- `cargo nextest run golden_embedding_matches_python`

### Wavecrate UI projection compatibility tests

Use for Wavecrate-owned projection snapshot and behavior changes. Default GUI
work should prefer focused tests around `src/native_app.rs` and the relevant
Radiant public API surface.

- targeted snapshot suites:
  - `startup_shot_matches_fixture`
  - `browser_dense_shot_matches_fixture`
  - `waveform_selection_shot_matches_fixture`
  - update fixtures with
    `cargo test -p wavecrate --lib update_shot_fixtures -- --ignored`

### Sample navigation responsiveness

Use this focused harness when changing sample selection, deferred loading,
waveform cache promotion, decode scheduling, or stale sample-load completion
handling:

- `cargo test -p wavecrate rapid_navigation_harness --no-default-features`

The harness exercises rapid keyboard navigation while sample-load business work
is queued, verifies visible selection updates before completions, checks stale
deferred/sample-load completions cannot overwrite the current selection, and
asserts Radiant runtime diagnostics report no slow UI update handlers.

Manual cold-cache smoke for large folders:

1. Run Wavecrate against a source folder with many uncached audio files.
2. Navigate quickly through the sample browser with keyboard arrows and mouse
   selection while cache/decode work is still cold.
3. Confirm row selection and status feedback move immediately.
4. Confirm late loads from earlier selections do not steal playback, selection,
   or visible error state from the current row.

### Radiant compatibility tests

Use for generic Radiant behavior and compatibility coverage in the vendored
runtime dependency.

- `cargo nextest run --manifest-path vendor/radiant/Cargo.toml`

### GUI test platform

Use for semantic GUI contracts and CLI scenarios.

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
  - this lane explicitly runs ignored fixture-backed GUI and bridge-runtime
    checks, including action parity and `contract_smoke_pack_runs_cleanly`; the
    default library test lane keeps only the cheaper runner and assertion
    checks.
- broader GUI suite:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 suite`

See `docs/TARGET.md` for the GUI artifact and automation contract details.

### Persistence profile boundary

Use these checks when the change could affect startup config loading/saving,
sample-source persistence, or GUI fixture profile selection.

- profile model:
  - live runs use `<config-base>/.wavecrate/`
  - sandbox/manual QA runs use `<config-base>/.wavecrate/profiles/sandbox/`
  - automated validation runs use `<config-base>/.wavecrate/profiles/automated-tests/`
- regression anchors:
  - `cargo test app_core::controller::tests::persistence_boundary::`
  - `cargo test gui_test::`
- wrapper coverage:
  - `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`
    now includes the persistence-boundary regression that snapshots the live
    `library.db` bytes before and after an isolated controller run
  - `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
    still covers the same regression through `cargo test -p wavecrate --lib`

### Windows manual confirmation

Use this after boundary/profile work when you want one explicit PowerShell proof
that validation did not leak fixture sources into the real startup profile.

1. Run `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`.
2. Run `powershell -ExecutionPolicy Bypass -File scripts/gui.ps1 contract`.
3. Start one normal live runtime without `WAVECRATE_CONFIG_HOME` or
   `WAVECRATE_CONFIG_PROFILE` overrides:
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

1. Start a repro run with the Wavecrate-owned debug flag:
   `cargo run --release -- --log`
   For a live run that also shows debug layout overlays, use either spelling:
   `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs debug-overlays`
   or `.\run.ps1 logs debug-layout`.
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
   the file; only enable `WAVECRATE_HOTPATH_TELEMETRY=1` when you deliberately need
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
- required nextest
- doc tests
- selected guardrail scripts
- advisory quarantined tests and hygiene checks
- local-only perf and GUI contract lanes where appropriate

When in doubt, run the wrapper script instead of assembling the command list by
hand.
