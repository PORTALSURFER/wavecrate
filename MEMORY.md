# Agent Memory

Last Updated: 2026-02-20T13:45:34Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing and shipping the Phase 6 segment-aware projection refresh
  milestone for runtime performance.
- In `src/app_core/native_bridge.rs`, I switched projection-cache miss handling
  from full reprojection to retained-model segment refresh (`status`,
  `browser frame`, `browser rows`, `map`, `waveform`) keyed per segment.
- In `src/app_core/native_bridge.rs`, I replaced full cache invalidation in
  high-frequency derived/wheel paths with key-only invalidation so retained
  segment state survives to the next pull.
- In `src/app_core/native_shell.rs`, I split browser projection into frame
  metadata and row-window helpers to support independent browser segment refresh.
- In `src/bin/bench/gui.rs` and `scripts/run_perf_guard.sh`, I added
  `interaction_segment_attribution` output/logging alongside existing stage
  attribution in perf reports.
- In `docs/plans/active/runtime_performance_exec_plan.md`, I added and checked
  off the Phase 6 milestone checklist.
- Full `bash scripts/ci_local.sh` is green after these changes.

## Work Notes

- Pending commit/push: Phase 6 segment-aware projection refresh and perf
  attribution updates.
