# Cleanup Plan (ROI Ranked)

Generated: 2026-02-27 (UTC)
Status legend: `[ ]` pending, `[x]` done

## Backlog

- [x] 1) Split `vendor/radiant/src/gui/native_shell/state.rs` into focused modules
  - ROI/Effort: High / L
  - Why it matters: This is the largest hotspot in the repo and mixes mutable interaction state, layout-adapter wiring, text truncation caches, hit-testing, animation, and frame paint generation in one file, making changes high-risk and reviews expensive.
  - Evidence: `vendor/radiant/src/gui/native_shell/state.rs` (~4740 LOC); mixed responsibilities visible in imports around lines 3-28 and `NativeShellState` around line 47.
  - Recommended change: Extract a `state/` module tree (`interaction.rs`, `paint_build.rs`, `hit_test.rs`, `truncation_cache.rs`, `animation.rs`) with a thin facade preserving current API.
  - Risk/tradeoffs: Medium; accidental behavior drift in input/paint coupling if extraction boundaries are wrong.
  - Suggested validation: `cargo test --manifest-path vendor/radiant/Cargo.toml`, native shell shot tests, then `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `radiant` commit `91fb246e`

- [x] 2) Decompose `vendor/radiant/src/gui_runtime/native_vello.rs` into runtime submodules
  - ROI/Effort: High / L
  - Why it matters: Runtime startup, event loop orchestration, invalidation routing, scene rebuild policy, profiling, text rendering, and action classification are concentrated in a single ~4k LOC file.
  - Evidence: `vendor/radiant/src/gui_runtime/native_vello.rs` (~3926 LOC); constants/env parsing at lines 49-105 and large mixed runtime logic throughout.
  - Recommended change: Extract `native_vello/{runner.rs,invalidation.rs,scene_rebuild.rs,input.rs,profiling.rs,repaint.rs}` while keeping entrypoints and behavior unchanged.
  - Risk/tradeoffs: Medium-high; this is performance-sensitive and redraw ordering is delicate.
  - Suggested validation: `cargo test --manifest-path vendor/radiant/Cargo.toml`, shot fixtures, `bash scripts/run_perf_guard.sh`, full `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `radiant` commit `5d82273c`

- [x] 3) Split `src/app_core/native_shell.rs` by projection domains
  - ROI/Effort: High / L
  - Why it matters: Core projection logic is centralized and long, which slows safe changes for browser/map/waveform/status projection independently.
  - Evidence: `src/app_core/native_shell.rs` (~1494 LOC); broad projection entrypoints start at `project_app_model` (around line 83) and continue across multiple domains.
  - Recommended change: Move browser/map/waveform/status/update projection into `src/app_core/native_shell/` modules; keep existing public facade functions stable.
  - Risk/tradeoffs: Medium; projection key assumptions and cache coupling must remain byte-equivalent.
  - Suggested validation: `cargo test --lib app_core::native_shell`, `cargo test --lib app_core::native_bridge`, then `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `3b85df24`

- [x] 4) Continue `jobs.rs` decomposition (issue gateway/token job runners)
  - ROI/Effort: High / M
  - Why it matters: `jobs.rs` remains large and central to async orchestration; issue-gateway/token worker code is still embedded and increases coupling.
  - Evidence: `src/app/controller/jobs.rs` (~1291 LOC), with issue-gateway/token message types and worker begin/clear methods in the same file.
  - Recommended change: Extract issue gateway/token worker runners into `src/app/controller/jobs/issue_gateway_jobs.rs` plus typed parameter structs for high-arity job builders.
  - Risk/tradeoffs: Medium; async cancellation and progress interactions can regress.
  - Suggested validation: targeted job-controller tests + `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `b8018a3a`

- [x] 5) Split recording waveform loader into queue, decode, and assembly modules
  - ROI/Effort: High / L
  - Why it matters: The recording loader combines queueing/state, decoding, bucket/analysis computation, and output shaping; this hurts maintainability in a hot path.
  - Evidence: `src/app/controller/playback/recording/waveform_loader.rs` (~1156 LOC); `RecordingWaveformState` and decode/aggregation logic start near lines 84+.
  - Recommended change: Extract `recording/waveform_loader/{queue.rs,decode.rs,aggregation.rs,result.rs}` and keep worker API unchanged.
  - Risk/tradeoffs: Medium; subtle waveform equivalence and incremental update behavior must not change.
  - Suggested validation: existing recording waveform tests + regression fixtures + `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `53cb2557`

- [x] 6) Complete browser search worker split (telemetry/queue/pipeline)
  - ROI/Effort: High / M
  - Why it matters: Search worker mixes cache invalidation, queue semantics, telemetry counters, and filtering/scoring pipeline in one module.
  - Evidence: `src/app/controller/library/wavs/browser_search_worker.rs` (~1140 LOC); queue/telemetry starts around lines 104-216.
  - Recommended change: Extract `browser_search_worker/{queue.rs,telemetry.rs,pipeline.rs,cache.rs}` preserving `SearchWorkerHandle` API and stale-generation behavior.
  - Risk/tradeoffs: Medium; high-frequency search latency and cancellation correctness are sensitive.
  - Suggested validation: browser search tests + perf guard + `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commits `ae1c39f3`, `8fcdd201`

- [x] 7) Split folder move drag-effects into planning/execution/result-apply layers
  - ROI/Effort: High / M
  - Why it matters: Drag/drop folder move logic currently mixes validation, job orchestration, filesystem operations, DB journaling, and status/reporting paths.
  - Evidence: `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` (~1105 LOC), with mixed concerns from handler entrypoints onward.
  - Recommended change: Extract `folder_moves/{plan.rs,worker.rs,apply_result.rs,journal.rs}` with explicit data contracts between layers.
  - Risk/tradeoffs: Medium; file-operation cancellation and rollback semantics must remain exact.
  - Suggested validation: folder drag/drop tests, file-op journal tests, `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `000edada`

- [x] 8) Replace `clippy::too_many_arguments` hotspots with typed parameter structs
  - ROI/Effort: High / M
  - Why it matters: Multiple core modules suppress argument-arity warnings, signaling high cognitive load and weak call-site clarity.
  - Evidence: file-level/line-level suppressions in `src/waveform/render.rs`, `src/waveform/render/cache.rs`, `src/app/controller/ui/map_view.rs`, `src/sample_sources/db/file_ops_journal.rs`, and others from audit scan.
  - Recommended change: Introduce cohesive input structs (`RenderParams`, `MovePlan`, `DbWriteOpts`, etc.) and migrate callsites incrementally.
  - Risk/tradeoffs: Low-medium; broad signature churn across modules.
  - Suggested validation: `cargo clippy --all-targets`, touched unit tests, `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `1fef4787`

- [x] 9) Refactor native-bridge metrics counter registry into grouped structs
  - ROI/Effort: Medium / M
  - Why it matters: Metrics module has very high static-counter density and repetitive logging math, increasing drift risk and edit friction.
  - Evidence: `src/app_core/native_bridge/metrics.rs` (~683 LOC), dense static counter block around lines 26-160 and long aggregation section around 201+.
  - Recommended change: Group counters into typed metric bundles and helper methods; keep feature-gated no-op fast path.
  - Risk/tradeoffs: Low-medium; refactor must preserve emitted metric names and cadence.
  - Suggested validation: native bridge metric tests + `bash scripts/run_perf_guard.sh` + full CI.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `d4762f89`

- [x] 10) Add focused unit tests for waveform transport selection/loop behavior
  - ROI/Effort: Medium / M
  - Why it matters: Complex transport selection/edit/loop interactions are implemented in a large file with no local test module, increasing regression risk.
  - Evidence: `src/app/controller/playback/transport.rs` (~643 LOC), no `#[cfg(test)]`/`mod tests` markers in file scan.
  - Recommended change: Add transport-focused tests (selection drag snapping, loop toggle side effects, seek debounce commit boundaries).
  - Risk/tradeoffs: Low; primarily test-only additions.
  - Suggested validation: `cargo test --lib transport`-focused filters + `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `c0da7163`

- [x] 11) Add focused tests for browser action commit/preview semantics
  - ROI/Effort: Medium / M
  - Why it matters: Browser focus/selection/playback-trigger semantics are subtle and frequently touched; local tests are sparse at the module boundary.
  - Evidence: `src/app/controller/library/wavs/browser_actions.rs` (~588 LOC), no local test module; commit-vs-preview methods around lines 34-257.
  - Recommended change: Add table-driven tests for delta focus, commit row behavior, playback request gating, and multi-select anchor updates.
  - Risk/tradeoffs: Low; expected behavior needs explicit fixtures.
  - Suggested validation: targeted browser/controller tests + `bash scripts/ci_local.sh`.
  - Completed: 2026-02-27 (UTC) - `sempal` commit `32c3367a`

- [ ] 12) Centralize truthy env parsing inside `radiant` runtime crate
  - ROI/Effort: Medium / S
  - Why it matters: `native_vello.rs` defines a local truthy parser; consolidating avoids token drift across runtime env flags in `radiant`.
  - Evidence: `vendor/radiant/src/gui_runtime/native_vello.rs` `parse_truthy_env` around lines 98-105.
  - Recommended change: Move parser to a shared `vendor/radiant/src/env_flags.rs` helper and update runtime callers.
  - Risk/tradeoffs: Low; behavior must remain token-compatible.
  - Suggested validation: add parser unit tests in `radiant`; run `cargo test --manifest-path vendor/radiant/Cargo.toml`.

- [ ] 13) Reduce `dead_code` suppressions by deleting or test-gating unused paths
  - ROI/Effort: Medium / M
  - Why it matters: High suppression count can hide stale code and increase maintenance burden.
  - Evidence: broad `#[allow(dead_code)]` usage across `src/` and `vendor/radiant/src/` from audit scan (many occurrences in layout/runtime modules and controller layers).
  - Recommended change: triage each suppression: delete dead code, gate to tests/features, or document explicit compatibility rationale.
  - Risk/tradeoffs: Low-medium; false positives if code is reflection/FFI/platform-conditional.
  - Suggested validation: `cargo clippy --all-targets`, platform-specific smoke checks, full CI.

- [ ] 14) Refresh quality scorecard review date and reconcile with current hotspots
  - ROI/Effort: Low / S
  - Why it matters: Current quality scorecard is stale and under-represents current large-file and perf-guard realities.
  - Evidence: `docs/QUALITY_SCORE.md` last reviewed date is `2026-02-18` with open known-gap bullets.
  - Recommended change: update review date/scores and cross-link active cleanup plan status.
  - Risk/tradeoffs: Low; documentation-only.
  - Suggested validation: docs lint/guardrails via `bash scripts/ci_local.sh`.

- [ ] 15) Add an automated cleanup audit script snapshot for repeatability
  - ROI/Effort: Low / S
  - Why it matters: This audit required ad-hoc commands; a scripted snapshot would reduce drift and speed future cleanup planning.
  - Evidence: current audit uses manual `rg/wc/allow` scans with no canonical report script in `scripts/`.
  - Recommended change: add `scripts/audit_cleanup_hotspots.sh` producing deterministic top-file/suppression/test-gap summary into `tmp/`.
  - Risk/tradeoffs: Low; minor maintenance overhead for script itself.
  - Suggested validation: script run in CI preflight or local docs runbook, then `bash scripts/ci_local.sh`.

## Progress Log

- Items 1-11 completed in strict ROI order and pushed.
- Next active item: 12) centralize truthy env parsing inside `radiant` runtime crate.
