# Build Speed

This note tracks the current Cargo package shape and the remaining structural
follow-up work for build-speed improvements.

## Current Shape

As of 2026-03-09, the repository is now a Cargo workspace.

The root package is still the shipping app package:

- package: `sempal`
- binary: `sempal`
- workspace default member: root package only

Support tools now live in separate workspace members instead of being
auto-discovered under the root package:

- `tools/bench-cli`
  - binary: `sempal-bench`
- `tools/similarity-prep`
  - binary: `sempal-similarity-prep`
- `tools/analysis-admin`
  - binaries:
    - `sempal-ann-rebuild`
    - `sempal-db-inspect`
    - `sempal-hdbscan`
    - `sempal-umap`
- `apps/updater-helper`
  - binary: `sempal-updater`
- `apps/installer`
  - binary: `sempal-installer`

The analysis pipeline now lives in its own shared workspace crate:

- `crates/sempal-analysis`
  - library crate: `sempal-analysis`
  - owns feature extraction, similarity embeddings, ANN index helpers, and
    similarity-map layout generation

The root package now sets `autobins = false` and explicitly declares only the
shipping app binary. That means normal root-level `cargo check --bins` and
`cargo nextest` runs no longer fan out across all support-tool binaries.

## Fast Local Loop

Use the narrowest check that still covers the code you touched.

App-focused lane:

- `bash scripts/devcheck_app.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck_app.ps1`
- runs `cargo check -p sempal --lib --bin sempal`

Analysis-focused lane:

- `cargo check -p sempal-analysis`
- use this first for isolated analysis pipeline edits before re-running the
  normal app lane

- `bash scripts/devcheck.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- runs `cargo check -p sempal --tests --bins`

- `bash scripts/ci_quick.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- runs `cargo nextest run -p sempal --profile quick --lib --tests`

Fast local runtime lane:

- `cargo run-fast -- <args>`
- equivalent to `cargo run -p sempal --bin sempal --profile release-local -- <args>`
- intended for repeated local runs where `cargo run -r` is too slow
- `release-local` keeps release-style codegen but enables incremental rebuilds,
  raises codegen units, and lowers optimization to `opt-level = 2`
- use plain `cargo run -r` when you need exact shipping-profile behavior

Workspace-wide lane:

- `bash scripts/devcheck_workspace.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck_workspace.ps1`
- runs `cargo check --workspace --tests --bins`

- `bash scripts/ci_quick_workspace.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/ci_quick_workspace.ps1`
- runs `cargo nextest run --workspace --profile quick --all-targets`

Full CI parity:

- `bash scripts/ci_local.sh`
- `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- runs workspace-wide clippy, tests, and doc tests

Recommended command matrix:

- Main app or runtime-only edits:
  - start with `devcheck_app`
  - then `devcheck`
- Analysis-only edits inside `crates/sempal-analysis`:
  - start with `cargo check -p sempal-analysis`
  - then `devcheck_app`
  - then `devcheck`
- Browser/runtime input, native shell, projection, controller work:
  - start with targeted `cargo test -p sempal --lib <name>`
  - then `devcheck`
- Support-tool bin changes:
  - run `cargo check -p <package-or-bin-owner>`
  - then `devcheck_workspace`
- Broader refactors, dependency work, package-shape edits:
  - go straight to `ci_quick_workspace`

## sccache

Repo Cargo scripts auto-use `sccache` when all of the following are true:

- `sccache` is installed and on `PATH`
- `RUSTC_WRAPPER` is not already set
- `SEMPAL_DISABLE_SCCACHE` is not `1`
- `sccache rustc --version` passes a short health probe

When repo PowerShell wrappers inherit `RUSTC_WRAPPER=sccache`, they now probe
that wrapper before running Cargo. If the probe fails or times out, the scripts
clear the inherited `sccache` wrapper for that run and fall back to direct
`rustc` instead of failing the whole validation lane immediately.

Install examples:

- `cargo install sccache --locked`
- Windows: `winget install Mozilla.sccache`

Useful follow-up commands:

- `sccache --show-stats`
- `sccache --zero-stats`

Expected benefit:

- big win on repeated local `cargo check` and `nextest` runs
- little or no benefit for the first full cold build
- no help for target fan-out by itself

## Why This Helps

The old root package mixed three roles:

- shipping app/runtime
- operational/admin tooling
- experimental/benchmark tooling

That forced broad commands like `cargo check --bins` to re-check many `sempal`
targets whenever shared library code changed.

The workspace split removes that target fan-out from the default app-focused
lane while preserving the same CLI names for support tools.

## Implemented Phase 1 Work

Completed:

1. added a top-level workspace
2. kept the root `sempal` app package and default app binary stable
3. split support-tool binaries into dedicated workspace members
4. updated local scripts so the default lane targets only the app package
5. added explicit workspace-wide smoke/test scripts for packaging and tooling work
6. updated CI and release commands to use workspace-wide coverage where needed

## Implemented Phase 2 Work

Completed on 2026-04-14:

1. extracted the analysis pipeline into `crates/sempal-analysis`
2. kept the root `sempal::analysis` surface as a compatibility facade
3. moved the heavy analysis dependency stack (`linfa*`, `hdbscan`, `hnsw_rs`,
   `rustfft`) off the root package manifest
4. updated analysis-admin binaries to consume the extracted crate directly
5. documented the narrower local loop for analysis-only edits

## Timings

Measured on Windows with:

- `cargo build -p sempal --bin sempal --profile release-local --timings`

Baseline before the extraction:

- timing report:
  [cargo-timing-20260414T201827.2585082Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T201827.2585082Z.html)
- total time: 549.8s (9m 9.8s)
- fresh units: 0
- dirty units: 603
- heaviest units included:
  - `hnsw_rs` 66.5s
  - `wgpu-hal` 66.3s
  - `wgpu-core` 65.8s
  - `wgpu` 36.3s
  - `vello` 36.1s
  - `cpal` build script 17.1s
  - `linfa-tsne` 11.1s

Follow-up build after the extraction:

- timing report:
  [cargo-timing-20260414T203441.1001366Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T203441.1001366Z.html)
- total time: 94.4s (1m 34.4s)
- fresh units: 601
- dirty units: 3
- `sempal` bin codegen/link step: 11.2s

Interpretation:

- the dependency graph is still dominated by the retained native graphics stack
  (`wgpu`/`vello`) and the Windows audio/tooling path (`cpal` + ASIO build
  script)
- the extracted analysis crate now rebuilds separately from the root app crate,
  which narrows app-only rebuilds after normal code edits
- the post-split number above is an incremental follow-up build, not a fresh
  cold rebuild, so it should be read as app-edit-loop evidence rather than a
  full cold-build replacement baseline

## Remaining Follow-Up

The highest-ROI package split is done, but the root library crate is still
broad. Only continue if timing data says the app package is still too slow.

Likely next shared-library candidates:

- `crates/sempal-library`
  - source DB access
  - scan helpers
  - sample metadata/state types used by admin tools
- `crates/sempal-updater-core`
  - updater download/apply/check logic shared by app and updater helper

Do not split GUI/runtime crates just to make the workspace compile. Only do the
next round after re-measuring.

## Re-measure Before More Refactors

After the workspace split:

- rerun warm timings for:
  - `cargo check -p sempal --tests --bins --timings`
  - `cargo check --workspace --tests --bins --timings`
- compare against the 2026-03-08 single-package baseline
- only continue splitting crates if the app package is still too broad

## Success Criteria

Treat this pass as successful only if the numbers move:

- warm `devcheck_app` stays comfortably below the old app-edit loop time
- warm `devcheck` drops materially because support-tool bins no longer recheck
- broad workspace checks remain available for packaging/tooling changes

## Non-Goals

These should still stay out of the first build-speed pass:

- rewriting runtime architecture
- splitting `audio`, `waveform`, `gui_runtime`, or `app_core` preemptively
- changing end-user CLI names
- broad dependency swaps without timing evidence
