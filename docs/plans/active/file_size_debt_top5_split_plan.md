# File Size Debt Top-5 Split Plan

Last updated (UTC): 2026-02-27T12:10:33Z
Owner: Codex agent sessions
Status: Active (incremental, behavior-preserving)

## Scope

Drive down file-size-budget debt on the 5 largest Rust hotspots first, with no
intentional behavior changes.

Baseline scan command:

```bash
find src -name '*.rs' -type f -print0 | xargs -0 wc -l | sort -nr | head -n 20
```

Baseline top 5 (2026-02-27):

1. `src/app_core/native_shell.rs` — 1494 LOC
2. `src/app/controller/jobs.rs` — 1466 LOC
3. `src/app/controller/playback/recording/waveform_loader.rs` — 1156 LOC
4. `src/app/controller/library/wavs/browser_search_worker.rs` — 1140 LOC
5. `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` — 1105 LOC

## Constraints

- No behavior changes.
- Preserve public/module API shapes unless a split requires re-export shims.
- Keep diff slices small enough for focused review.
- Keep performance-sensitive paths explicit; avoid indirection-heavy abstractions.

## Incremental Split Queue

### 1) `src/app/controller/jobs.rs`

Target split tree:

- `jobs/progress_reporting.rs` (done)
- `jobs/queue_orchestration.rs` (done)
- `jobs/retry_policy.rs` (done)
- `jobs/source_db_maintenance.rs` (in progress)
- `jobs/normalization_worker.rs` (in progress)
- next: extract issue gateway/token job runners into `jobs/issue_gateway_jobs.rs`

Acceptance:

- `jobs.rs` keeps orchestration and state ownership only.
- worker-specific logic lives in submodules.

### 2) `src/app/controller/library/wavs/browser_search_worker.rs`

Planned slices:

- extract queue telemetry counters/emitters into `wavs/browser_search_worker/telemetry.rs`
- extract queue state primitives into `wavs/browser_search_worker/queue.rs`
- extract scoring/filter pipeline helpers into `wavs/browser_search_worker/pipeline.rs`

Acceptance:

- keep `SearchWorkerHandle` API unchanged.
- keep cancellation/generation semantics byte-for-byte equivalent.

### 3) `src/app/controller/playback/recording/waveform_loader.rs`

Planned slices:

- split request coalescing + stale-drop flow from decode/render path.
- split telemetry/counter helpers from IO/decode routines.
- split output/result assembly into focused module.

Acceptance:

- preserve request-id gating and cancellation behavior.
- preserve existing loader test expectations.

### 4) `src/app_core/native_shell.rs`

Planned slices:

- move sectioned projection helpers into `native_shell/` module tree:
  - browser projection
  - waveform projection
  - map projection
  - status/update projection
- keep façade functions in `native_shell.rs` re-exporting stable entrypoints.

Acceptance:

- no change to `app_core` callers.
- no change to native projection payload shapes.

### 5) `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs`

Planned slices:

- split planning/validation, filesystem execution, and db-update mapping.
- isolate path-rewrite utilities into helper module.

Acceptance:

- preserve drag/drop error aggregation and cancellation semantics.
- preserve move journaling payload shapes.

## Execution Cadence

For each slice:

1. Move one cohesive chunk (roughly 100-300 LOC).
2. Keep signatures stable via re-exports/adapters.
3. Run targeted tests for touched area.
4. Run `bash scripts/ci_local.sh`.
5. Run `bash scripts/run_perf_guard.sh` when touching hot-path/perf-sensitive code.
6. Commit/push one coherent behavior-preserving change.

## Validation Policy

Required per slice:

- `bash scripts/ci_local.sh`

Required for perf-sensitive slices:

- `bash scripts/run_perf_guard.sh`
- confirm no new warning/fail deltas compared to previous commit’s perf-guard summary

## Current Pass Outcome

- Added/used shared `jobs` submodules without behavior changes:
  - queue orchestration
  - progress reporting
  - retry policy
  - source-db maintenance (extracted)
  - normalization worker (extracted)

Next immediate slice: extract issue-gateway/token worker runners from `jobs.rs`.
