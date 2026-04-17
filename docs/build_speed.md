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

The source-db and library-storage core now live in their own shared workspace
crate:

- `crates/sempal-library`
  - library crate: `sempal-library`
  - owns application-directory helpers, SQLite extension loading, per-source DB
    storage, and global library DB storage

The scan-layer logic now lives in its own shared workspace crate:

- `crates/sempal-scan`
  - library crate: `sempal-scan`
  - owns source scanning and scan-state helpers built on top of
    `sempal-library`

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

Library-storage lane:

- `cargo check -p sempal-library`
- use this first for source-db, file-journal, library DB, and related storage
  helper edits before re-running the normal app lane

Scan-focused lane:

- `cargo check -p sempal-scan`
- use this first for source scanning, deep-hash, rename reconciliation, and
  scan-state edits before re-running the normal app lane

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

Windows-only linker experiment lane:

- `cargo --config .cargo/config.release-local-lld.toml build -p sempal --bin sempal --profile release-local --timings`
- opt-in only; keeps the repo default linker unchanged unless you pass the
  extra config override explicitly
- use only for measurement or local comparison; the current measured result is
  not recommended as the default fast lane

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
- Storage-only edits inside `crates/sempal-library`:
  - start with `cargo check -p sempal-library`
  - then `devcheck_app`
  - then `devcheck`
- Scan-only edits inside `crates/sempal-scan`:
  - start with `cargo check -p sempal-scan`
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

## Implemented Phase 3 Work

Completed on 2026-04-14:

1. extracted application-directory helpers, SQLite extension loading, and the
   source-db/library-storage core into `crates/sempal-library`
2. kept the root `sempal::sample_sources` surface as a compatibility facade for
   config, scanner, and scan-state code
3. rewired the root crate so `app_dirs` and `sqlite_ext` are re-exported from
   the shared storage crate
4. left app/controller orchestration and analysis-job workflows in `sempal`
5. documented the narrower local loop for storage-only edits

## Implemented Phase 4 Work

Completed on 2026-04-17:

1. extracted `sample_sources::scanner` and `sample_sources::scan_state` into
   `crates/sempal-scan`
2. kept the root `sempal::sample_sources` surface as a compatibility facade for
   config plus re-exported scan/storage helpers
3. kept app config, updater logic, GUI/runtime code, and controller
   orchestration in the root crate
4. moved the scan-layer unit tests with the extracted crate
5. documented the narrower local loop for scan-only edits

## Implemented Phase 5 Work

Completed on 2026-04-17:

1. added an opt-in Windows `lld-link` override config for the `release-local`
   developer lane
2. re-measured the baseline `release-local` timing matrix on the same machine
3. re-ran the same timing matrix with the `lld-link` override enabled
4. documented the exact experiment commands and the measured outcome

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

Fresh rebuild after the extraction (`cargo clean` first):

- timing report:
  [cargo-timing-20260414T204640.7742215Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T204640.7742215Z.html)
- total time: 233.7s (3m 53.7s)
- fresh units: 0
- dirty units: 604
- heaviest units included:
  - `sempal` 74.0s
  - `libsqlite3-sys` build script (run) 66.8s
  - `asio-sys` build script (run) 63.5s
  - `wgpu-core` 51.0s
  - `sempal-analysis` 39.2s
  - `rav1e` 33.2s
  - `radiant` 26.8s
  - `sempal` bin `sempal` 9.8s

Warm rebuild after a tiny app-only edit (`src/main.rs`):

- timing report:
  [cargo-timing-20260414T205050.9207467Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T205050.9207467Z.html)
- total time: 5.6s
- fresh units: 603
- dirty units: 1
- rebuilt units:
  - `sempal` bin `sempal` 4.3s

Warm rebuild after a tiny `sempal-analysis` edit:

- timing report:
  [cargo-timing-20260414T205102.518673Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T205102.518673Z.html)
- total time: 25.0s
- fresh units: 601
- dirty units: 3
- rebuilt units:
  - `sempal-analysis` 3.2s
  - `sempal` 18.3s
  - `sempal` bin `sempal` 4.8s

Decision readout:

- the extraction materially improved the clean app build: the fresh
  `release-local` app build dropped from 549.8s to 233.7s
- the final app binary step is no longer the dominant clean-build cost:
  `sempal` bin is 9.8s out of 233.7s
- app-entry edits are now cheap: a tiny `src/main.rs` change rebuilt only the
  binary and finished in 5.6s
- analysis-layer edits still fan back into the broad root `sempal` crate:
  changing `sempal-analysis` triggered a 3.2s analysis rebuild, then an 18.3s
  root-crate rebuild, then the 4.8s app binary step
- native/non-Rust work still matters for fresh builds, but mainly in the cold
  path rather than the tail: `libsqlite3-sys` and `asio-sys` build scripts are
  both larger than the final binary step in the fresh timing report

That Phase 2 measurement led directly to the Phase 3 `sempal-library`
extraction recorded below.

Fresh rebuild after the library-storage extraction (`cargo clean` first):

- timing report:
  [cargo-timing-20260414T210707.0330992Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T210707.0330992Z.html)
- total time: 215.1s (3m 35.1s)
- fresh units: 0
- dirty units: 605
- heaviest units included:
  - `sempal` 74.5s
  - `libsqlite3-sys` build script (run) 62.2s
  - `asio-sys` build script (run) 50.0s
  - `wgpu-core` 49.1s
  - `sempal` bin `sempal` 10.0s
  - `sempal-library` 7.6s

Warm rebuild after a tiny app-only edit (`src/main.rs`):

- timing report:
  [cargo-timing-20260414T211051.4097967Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T211051.4097967Z.html)
- total time: 5.3s
- fresh units: 604
- dirty units: 1
- rebuilt units:
  - `sempal` bin `sempal` 4.3s

Warm rebuild after a tiny `sempal-library` edit:

- timing report:
  [cargo-timing-20260414T211108.9486715Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260414T211108.9486715Z.html)
- total time: 19.4s
- fresh units: 602
- dirty units: 3
- rebuilt units:
  - `sempal` 14.2s
  - `sempal` bin `sempal` 3.7s
  - `sempal-library` 1.4s

Interpretation after Phase 3:

- the storage extraction improved the fresh app build again, but modestly:
  233.7s down to 215.1s
- app-only warm rebuilds remain in the cheap 5s lane
- storage-layer edits are now cheaper than the pre-split `sempal-analysis`
  edit path (19.4s vs 25.0s), but they still force a broad root `sempal`
  rebuild
- the final binary step remains small relative to the root crate rebuild after
  storage edits, so linker tuning still does not look like the main next win

That Phase 3 measurement led directly to the Phase 4 `sempal-scan` extraction
recorded below.

Fresh rebuild after the scan extraction (`cargo clean` first):

- timing report:
  [cargo-timing-20260417T090051.2970558Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T090051.2970558Z.html)
- total wall time: 212.4s (3m 32.4s)
- fresh units: 0
- dirty units: 606
- heaviest units included:
  - `libsqlite3-sys` build script (run) 65.6s
  - `sempal` 59.1s
  - `wgpu-core` 47.1s
  - `asio-sys` build script (run) 44.5s
  - `sempal-analysis` 37.6s
  - `sempal-scan` 3.5s

Warm rebuild after a tiny app-only edit (`src/main.rs`):

- timing report:
  [cargo-timing-20260417T090437.8041759Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T090437.8041759Z.html)
- total time: 5.1s
- fresh units: 605
- dirty units: 1
- rebuilt units:
  - `sempal` bin `sempal` 3.8s

Warm rebuild after a tiny `sempal-scan` edit:

- timing report:
  [cargo-timing-20260417T090453.1366429Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T090453.1366429Z.html)
- total time: 18.7s
- fresh units: 603
- dirty units: 3
- rebuilt units:
  - `sempal` 13.8s
  - `sempal` bin `sempal` 3.3s
  - `sempal-scan` 1.0s

Interpretation after Phase 4:

- the scan extraction improved the dependency-edit path again, but only
  modestly: 19.4s down to 18.7s
- app-only warm rebuilds remain in the cheap 5s lane
- the fresh app build stayed roughly flat after this split: 215.1s down to
  212.4s
- the remaining warm rebuild cost is still the broad root `sempal` crate
  rather than the final binary step
- linker tuning still looks secondary to root-crate narrowing because the
  binary step is 3.3s while the root crate rebuild is 13.8s

That Phase 4 measurement led directly to the Phase 5 linker experiment
recorded below.

Baseline re-measure on the current tree (default MSVC linker):

Fresh rebuild after Phase 4 (`cargo clean --profile release-local` first):

- timing report:
  [cargo-timing-20260417T093453.8826125Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T093453.8826125Z.html)
- total time: 199.8s (3m 19.8s)
- fresh units: 0
- dirty units: 606
- heaviest units included:
  - `libsqlite3-sys` build script (run) 68.9s
  - `sempal` 63.9s
  - `wgpu-core` 49.8s
  - `asio-sys` build script (run) 47.6s

Warm rebuild after a tiny app-only edit (`src/main.rs`):

- timing report:
  [cargo-timing-20260417T093828.6347778Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T093828.6347778Z.html)
- total time: 5.6s
- fresh units: 605
- dirty units: 1
- rebuilt units:
  - `sempal` bin `sempal` 4.5s

Warm rebuild after a tiny `sempal-scan` edit:

- timing report:
  [cargo-timing-20260417T093847.5090456Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T093847.5090456Z.html)
- total time: 25.2s
- fresh units: 603
- dirty units: 3
- rebuilt units:
  - `sempal` 19.1s
  - `sempal` bin `sempal` 4.6s
  - `sempal-scan` 1.6s

Experimental `lld-link` lane on the same machine:

Exact command shape:

- `cargo --config .cargo/config.release-local-lld.toml build -p sempal --bin sempal --profile release-local --timings`

Fresh rebuild with `lld-link` (`cargo clean --profile release-local` first):

- timing report:
  [cargo-timing-20260417T095009.8214348Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T095009.8214348Z.html)
- total time: 547.0s (9m 7.0s)
- fresh units: 0
- dirty units: 606
- heaviest units included:
  - `windows` 0.62.2 152.8s
  - `windows` 0.61.3 140.8s
  - `naga` 104.9s
  - `hnsw_rs` 93.1s

Warm rebuild after a tiny app-only edit (`src/main.rs`):

- timing report:
  [cargo-timing-20260417T095925.2028885Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T095925.2028885Z.html)
- total time: 5.0s
- fresh units: 605
- dirty units: 1
- rebuilt units:
  - `sempal` bin `sempal` 3.6s

Warm rebuild after a tiny `sempal-scan` edit:

- timing report:
  [cargo-timing-20260417T095937.3866677Z.html](/C:/dev/sempal/target/cargo-timings/cargo-timing-20260417T095937.3866677Z.html)
- total time: 18.8s
- fresh units: 603
- dirty units: 3
- rebuilt units:
  - `sempal` 14.5s
  - `sempal` bin `sempal` 3.0s
  - `sempal-scan` 1.2s

Interpretation after Phase 5:

- `lld-link` improved the warm binary step modestly on this machine:
  4.5s down to 3.6s for the app-only edit path
- `lld-link` also improved the dependency-edit warm rebuild:
  25.2s down to 18.8s
- but the fresh `release-local` build regressed drastically:
  199.8s up to 547.0s
- because the fresh-build regression is far larger than the warm-build win, the
  `lld-link` lane is not recommended as the default local fast-build path for
  this repo on this machine
- keep the default `release-local` lane as the recommended workflow unless a
  future toolchain update changes this result materially

## Remaining Follow-Up

The three highest-ROI analysis/storage/scan splits are now done, and the first
linker experiment produced a mixed result that is not worth adopting broadly.
Only continue if timing data says the remaining root rebuild is still too
expensive.

Likely next candidates:

- app-config extraction only if the remaining `sample_sources::config` boundary
  can be separated cleanly from root runtime concerns
- a focused updater-core split if updater work begins to dominate app rebuilds
- another linker experiment only if a toolchain upgrade or a very narrow
  debug-info tweak changes the tradeoff materially

Do not split GUI/runtime crates just to make the workspace compile. Only do the
next round after re-measuring.

## Re-measure Before More Refactors

After the scan split:

- rerun the `release-local` timing matrix:
  - fresh `cargo build -p sempal --bin sempal --profile release-local --timings`
  - tiny app-only warm rebuild
  - tiny dependency-layer warm rebuild in the next candidate crate
- rerun the normal app compile/test lanes
- only continue splitting crates if the root `sempal` rebuild still dominates
  the dependency-edit path

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
