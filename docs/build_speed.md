# Build Speed

This note captures the current local compile workflow and the highest-ROI
structural follow-up for reducing Cargo fan-out.

## Fast Loop

Use the narrowest check that still covers the code you touched:

- App-only compile gate:
  - `bash scripts/devcheck_app.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck_app.ps1`
  - Checks `cargo check --lib --bin sempal`.
  - Best when editing the main app/runtime and not touching support-tool bins or tests.
- Required smoke gate:
  - `bash scripts/devcheck.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - Checks `cargo check --tests --bins`.
- Fast test gate:
  - `bash scripts/ci_quick.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

## sccache

Repo Cargo scripts now auto-use `sccache` when all of the following are true:

- `sccache` is installed and on `PATH`
- `RUSTC_WRAPPER` is not already set
- `SEMPAL_DISABLE_SCCACHE` is not `1`

Install examples:

- `cargo install sccache --locked`
- Windows: `winget install Mozilla.sccache`

Useful follow-up commands:

- `sccache --show-stats`
- `sccache --zero-stats`

## Why Builds Still Fan Out

On 2026-03-08, a warm `cargo check --tests --bins --timings` run still took
about two minutes and spent most of that time rechecking multiple `sempal`
targets, not third-party dependencies. The main reason is target fan-out from
one large package:

- app binary: [`main.rs`](/C:/dev/sempal/src/main.rs)
- support bins in [`src/bin`](/C:/dev/sempal/src/bin)
- test and integration targets

When the root package changes, Cargo rechecks the shared `sempal` library plus
each dependent binary/test target.

## Crate Split Sketch

Highest-ROI first split: move standalone support tools out of the root package
into separate workspace packages while keeping the shipping app path stable.

### Phase 1: Separate support-tool packages

Create a workspace and move these out of the root `sempal` package:

- `tools/analysis-admin`
  - `sempal-ann-rebuild`
  - `sempal-db-inspect`
  - `sempal-hdbscan`
  - `sempal-umap`
- `tools/similarity-prep`
  - `sempal-similarity-prep`
- `tools/bench-cli`
  - `sempal-bench`
- `apps/updater-helper`
  - `sempal-updater`
- `apps/installer`
  - `src/bin/sempal-installer`

Why this is the first split:

- These targets are operationally separate from the main app.
- They currently force extra root-package target checks during `--bins` and
  broad cargo operations.
- The move is mostly packaging/workspace work, not domain refactoring.

### Phase 2: Extract narrower shared libraries only where needed

After Phase 1, split shared code by domain if compile fan-out is still too
large:

- `crates/sempal-analysis`
  - ANN, embedding, UMAP/t-SNE, vector/blob helpers
- `crates/sempal-library`
  - sample-source DB access and scan/state helpers needed by admin tools
- `crates/sempal-updater-core`
  - updater logic shared by app + updater helper

Keep the GUI/runtime path together until the packaging split is done. Breaking
up `app_core`, `gui_runtime`, `audio`, and `waveform` too early increases risk
without the same immediate compile-time payoff.

## Suggested Execution Order

1. Keep using `devcheck_app` plus `sccache` for immediate local relief.
2. Convert the repo to a workspace while preserving the current `sempal`
   package and binary names.
3. Move support-tool bins into separate workspace packages with unchanged CLI
   behavior.
4. Re-run Cargo timings and only then decide whether domain-library extraction
   is still worth the complexity.
