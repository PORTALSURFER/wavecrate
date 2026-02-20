# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-20T21:40:48Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).

## Next tasks (ordered)

1. Validate Phase 7 item 1 with perf/profiler evidence and capture before/after
   static-scene rebuild behavior.
2. Run perf guard/profiler captures focused on
   `waveform_pan_zoom_adjacent_latency` and record before/after metrics for the
   new waveform cache/delta path.
3. Keep static segment rebuilds fully dirty-mask-driven in radiant frame
   segments and add targeted action-to-segment tests.
4. Update `MEMORY.md` and this queue in the same commit as each milestone.

## Done recently

- Completed ROI item #1: switched waveform multi-step zoom to single-pass math with regression coverage.
- Completed Phase 7 item 1 foundation: tightened radiant invalidation scope routing so
  high-frequency browser/search/prompt actions use model+overlay invalidation.
- Completed Phase 7 waveform cache-hit improvement: quantized waveform view
  metadata matching and stabilized texture-width bucketing.
- Completed Phase 7 delta pan milestone: added waveform translation reuse
  (shift + edge patch render) with adjacent-pan regression coverage.
