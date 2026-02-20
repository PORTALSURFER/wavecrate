# Agent Memory

Last Updated: 2026-02-20T12:53:15Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the Phase 5 runtime projection optimizations for browser
  and waveform interaction responsiveness.
- In `src/app_core/native_shell.rs`, I added retained browser projection caches
  for static row fields and selected-path lookups, keyed by visible-row
  revision/signatures to avoid redundant per-row recompute while navigating.
- In `src/app_core/native_bridge.rs`, I expanded projection cache keys for
  browser selection and waveform overlay/view state, moved queued waveform dirty
  marking to flush-time with no-op key-diff skipping, and split waveform dirty
  reasons into view vs overlay to skip unnecessary waveform image refreshes.
- In `src/app_core/native_bridge.rs`, I added bridge perf attribution counters
  for projection-cache hit/miss and waveform-image refresh apply/skip.
- In `src/app/controller.rs`, I added retained controller-side projection cache
  fields used by native shell projection.
- `bash scripts/ci_local.sh` is green after these changes, and the perf guard is
  now fully within warning limits on this run.

## Work Notes

- Pending commits (not yet pushed): Phase 5 projection caching, waveform
  dirty/invalidation tightening, and bridge profiling counter expansion.
