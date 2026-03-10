# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-10T16:49:18Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` are complete.
- The active ordered cleanup backlog lives in `tmp/cleanup_plan.md`, and Phase 2 is in progress.

## Next tasks (ordered)

1. Continue cleanup item 16 from `tmp/cleanup_plan.md` in strict ROI order.
2. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized on the next cleanup or perf milestone.
3. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a new perf follow-up lane is opened after cleanup.
