# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-20T21:07:18Z
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
2. Implement waveform adjacent pan/zoom cache-hit improvements
   (meta quantization + stable texture-width buckets).
3. Implement a delta waveform translation path that reuses prior image data for
   adjacent pan steps.
4. Update `MEMORY.md` and this queue in the same commit as each milestone.

## Done recently

- Completed ROI item #1: switched waveform multi-step zoom to single-pass math with regression coverage.
- Completed Phase 7 item 1 foundation: tightened radiant invalidation scope routing so
  high-frequency browser/search/prompt actions use model+overlay invalidation.
