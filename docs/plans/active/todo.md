# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-01T22:25:32Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- File-size debt burn-down for top 5 Rust hotspots (behavior-preserving splits).

## Next tasks (ordered)

1. Execute `tmp/perf_plan.md` item 5:
   reduce retained-model clone churn on projection misses.
2. Execute `tmp/perf_plan.md` item 6:
   remove full status-bar formatting from motion-model projection hot path.
3. Execute `tmp/perf_plan.md` item 7:
   cache/carry waveform upload blobs across draws to reduce image upload churn.
4. Keep handoff docs synchronized at each milestone:
   update `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` in the same cycle.
