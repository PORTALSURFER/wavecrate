# Runtime and Data Contracts

This document collects the durable technical contracts that used to be spread
across many narrow notes. Keep detailed plans in `tmp/` and keep this file for
current, behavioral contracts only.

## Native bridge projection cache

`src/app_core/native_bridge/**` owns the retained native projection model.

Core rules:

- pulls should reuse retained segments when the relevant projection keys match
- invalidation should stay as targeted as correctness allows
- overlay-only waveform edits should avoid unnecessary waveform-image rebuilds
- local-only pull shortcuts are conservative and must never bypass required
  derived invalidation

Use the bridge metrics helpers instead of ad-hoc logging when profiling:

- `measure_projection_segment_lookup_counts`
- `measure_projection_segment_probe`
- `measure_projection_rebuild_cause_counts`

Behavioral coverage anchor:

- `src/app_core/native_bridge/tests.rs`

## GUI test platform

The GUI test platform has four layers:

1. host action catalog in `src/app_core/actions/**`
2. semantic automation snapshot emitted by the native shell
3. deterministic GUI test mode and artifact emission
4. in-process scenario runner and `tools/gui-test-cli`

The important contract is semantic stability:

- target controls by stable node ids and action ids
- prefer semantic automation over screenshot matching when possible
- correlate GUI artifacts with run-contract metadata instead of inventing a
  separate reporting format

Current development loops:

- semantic contract lane: `scripts/gui.ps1 contract`
- broader suite: `scripts/gui.ps1 suite`
- local desktop AIV smoke/suite:
  - `scripts/gui.ps1 aiv-smoke`
  - `scripts/gui.ps1 aiv-suite -PackName desktop-regression`

Desktop AIV remains local-only until foreground/focus stability is strong
enough for promotion.

## Run artifacts

Run outputs should remain machine-readable and deterministic.

Artifacts include:

- run manifest metadata
- GUI test artifacts
- optional bug-bundle outputs
- perf-guard outputs where relevant

When adding a new run artifact, reuse the existing run metadata model instead
of inventing a parallel schema.

## File-operation recovery

`src/sample_sources/db/file_ops_journal.rs` owns copy/move crash recovery.

Durability rules:

1. write journal intent before assuming filesystem mutation
2. advance journal stages only after each durable boundary completes
3. reconcile by checking filesystem state again instead of trusting the journal
   blindly
4. prefer data preservation when observed state is ambiguous

## Folder-delete recovery

`src/app/controller/library/source_folders/delete_recovery/**` owns retained
folder-delete recovery.

Contract:

- staged deletes move data into an app-owned trash area
- startup recovery restores incomplete deletes conservatively
- fully committed deletes remain recoverable until explicit restore or purge
- explicit restore merges carefully and keeps both copies when content differs

## Updater policy

Updater behavior is intentionally conservative.

- install paths must not traverse unsafe symlink paths
- Windows release installs are manual by design
- development-only overrides belong behind explicit env vars
- updater failures should preserve the installed app rather than forcing risky
  writes

See `docs/TROUBLESHOOTING.md` and `docs/ENV_VARS.md` for diagnostics and
overrides.

## Data-format notes

Keep these formats stable unless there is a coordinated migration:

- feature-vector payloads used by the similarity pipeline
- ANN container artifacts used for search/index assets

When either format changes:

- document the migration at the owning code boundary
- update regression or fixture coverage
- avoid version drift between generated assets and the reader

## Performance posture

Performance-sensitive work should preserve these expectations:

- the app stays responsive under large-library workloads
- bridge and waveform hot paths remain measurable and testable
- performance checks use the existing wrapper scripts and reproducible datasets
  instead of one-off local measurements

Historical execution diaries and phase logs belong in `tmp/`, not here.
