# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-01T21:59:57Z
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

1. Execute `tmp/perf_plan.md` item 3:
   stage transient detection after load/play result delivery so playback
   responsiveness stays immediate under rapid focus churn.
2. Execute `tmp/perf_plan.md` item 4:
   make audio read path stale-aware with chunked cancellation checks.
3. Execute `tmp/perf_plan.md` item 5:
   reduce retained-model clone churn on projection misses.
4. Keep handoff docs synchronized at each milestone:
   update `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` in the same cycle.
