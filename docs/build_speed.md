# Build Speed

This note captures the current local compile workflow and the next structural
steps that should reduce Cargo fan-out in a measurable way.

## Current Shape

The root package in [Cargo.toml](/C:/dev/sempal/Cargo.toml) is still one large
crate named `sempal`. A normal library edit can fan out into:

- main app binary: [main.rs](/C:/dev/sempal/src/main.rs)
- grouped bin packages under [`src/bin/bench`](/C:/dev/sempal/src/bin/bench),
  [`src/bin/sempal-installer`](/C:/dev/sempal/src/bin/sempal-installer), and
  [`src/bin/sempal-updater`](/C:/dev/sempal/src/bin/sempal-updater)
- standalone bin targets in [`src/bin`](/C:/dev/sempal/src/bin):
  - `sempal-ann-rebuild.rs`
  - `sempal-bench.rs`
  - `sempal-db-inspect.rs`
  - `sempal-hdbscan.rs`
  - `sempal-similarity-prep.rs`
  - `sempal-umap.rs`
  - `sempal-updater.rs`
- library tests and integration-style test targets

On 2026-03-08, a warm `cargo check --tests --bins --timings` run still took
about two minutes. The expensive part was not dependency compilation. It was
re-checking many `sempal` targets that all depend on the same root crate.

## Fast Local Loop

Use the narrowest check that still covers the code you touched:

- App-only compile gate:
  - `bash scripts/devcheck_app.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck_app.ps1`
  - Runs `cargo check --lib --bin sempal`
- Required smoke gate:
  - `bash scripts/devcheck.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - Runs `cargo check --tests --bins`
- Fast test gate:
  - `bash scripts/ci_quick.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - Runs `cargo nextest run --profile quick --lib --tests`

Recommended command matrix:

- Main app or runtime-only edits:
  - start with `devcheck_app`
  - then `devcheck`
- Browser/runtime input, native shell, projection, controller work:
  - start with targeted `cargo test --lib <name>`
  - then `devcheck`
- Support-tool bin changes:
  - run `cargo check --bin <target>`
  - then `devcheck`
- Broader refactors, dependency work, build-system edits:
  - go straight to `ci_quick`

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

Expected benefit:

- big win on repeated local `cargo check` and `nextest` runs
- little or no benefit for the first full cold build
- no help for target fan-out by itself

## Why The Current Package Is Slow

The root package currently mixes three roles:

- shipping app/runtime
- operational/admin tooling
- experimental/benchmark tooling

That is the wrong dependency boundary for compile speed. `cargo check --bins`
and broad test commands treat all those targets as part of one package, so a
single shared-lib change causes many target rechecks.

The immediate problem is package shape, not low-level Rust compiler tuning.

## Highest-ROI Structural Change

The best next step is to convert the repo into a workspace and move the
support-tool binaries out of the root `sempal` package first.

### Phase 1: Split Off Support Tools

Create workspace members with the same CLI names but separate packages:

- `apps/sempal-app`
  - owns the shipping app binary and the main runtime path
  - keeps package name `sempal`
  - keeps `default-run = "sempal"`
- `tools/analysis-admin`
  - owns `sempal-ann-rebuild`
  - owns `sempal-db-inspect`
  - owns `sempal-hdbscan`
  - owns `sempal-umap`
- `tools/similarity-prep`
  - owns `sempal-similarity-prep`
- `tools/bench-cli`
  - owns `sempal-bench`
  - owns the code now grouped under [`src/bin/bench`](/C:/dev/sempal/src/bin/bench)
- `apps/updater-helper`
  - owns `sempal-updater`
  - owns the code now grouped under
    [`src/bin/sempal-updater`](/C:/dev/sempal/src/bin/sempal-updater)
- `apps/installer`
  - owns the code now grouped under
    [`src/bin/sempal-installer`](/C:/dev/sempal/src/bin/sempal-installer)

Why this split is first:

- it reduces root-package target count without deep domain surgery
- it preserves the app path users actually run
- it is mostly Cargo/workspace plumbing plus import cleanup
- it gives measurable speed results before harder architectural changes

### Phase 1 Boundaries

Keep these boundaries explicit:

- app package owns GUI/runtime-specific code paths
- support tools should depend on shared libraries, not on app binaries
- tool packages should not pull in GUI-only dependencies unless required
- do not move `vendor/radiant` or native runtime code during this phase

### Phase 1 Likely Shared Libraries

Once bins move out, introduce only the minimum shared crates needed to compile:

- `crates/sempal-analysis`
  - feature extraction
  - embeddings
  - ANN helpers
  - UMAP/t-SNE support
- `crates/sempal-library`
  - source DB access
  - scan helpers
  - sample metadata/state types used by admin tools
- `crates/sempal-updater-core`
  - updater download/apply/check logic shared by app and updater helper

Do not split GUI/runtime crates just to make the workspace compile. That adds
coordination cost before the high-ROI package split is complete.

## Execution Plan

### Step 1: Workspace Conversion

- add a top-level `[workspace]`
- keep the current root package buildable during transition
- preserve binary names and current default app entrypoint
- move package-specific dependencies down to the packages that actually use them

### Step 2: Move Tool Targets

Move one tool package at a time:

1. `tools/bench-cli`
2. `tools/similarity-prep`
3. `tools/analysis-admin`
4. `apps/updater-helper`
5. `apps/installer`

That order is deliberate:

- bench and similarity prep are lower-risk than installer/runtime helpers
- updater and installer are more likely to touch app-adjacent integration code

### Step 3: Update Local Scripts

After the workspace exists, adjust script behavior:

- `devcheck_app` should keep targeting only the shipping app package
- `devcheck` should stay broad enough for normal app work, but avoid unrelated
  tool packages by default if possible
- add an explicit “all workspace packages” script if full-package coverage is
  still needed locally

The goal is two lanes:

- fast app lane for normal UI/runtime edits
- broad workspace lane for packaging/tooling changes

### Step 4: Re-measure Before More Refactors

After the package split:

- rerun `cargo check --tests --bins --timings`
- compare warm timings against the 2026-03-08 baseline
- only continue splitting crates if the app package is still too broad

## Success Criteria

Treat this as successful only if the numbers move:

- warm `devcheck_app` stays comfortably below current app-edit loop time
- warm `devcheck` drops materially because support-tool bins no longer recheck
- broad workspace checks become more predictable because unrelated packages stop
  recompiling with every app edit

Practical acceptance target for Phase 1:

- reduce warm app-focused smoke checks by at least 30 to 45 percent compared to
  the current single-package baseline

## Non-Goals

These should not be bundled into the first build-speed pass:

- rewriting runtime architecture
- splitting `audio`, `waveform`, `gui_runtime`, or `app_core` preemptively
- changing end-user CLI names
- broad dependency swaps without timing evidence

## Suggested Next Move

If the next task is implementation rather than more planning, start with:

1. add `[workspace]` while keeping the current app package name and binary name stable
2. extract `tools/bench-cli` first
3. rerun timings
4. continue only if the numbers justify the next package move
