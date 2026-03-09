# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-09T19:03:54Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` are complete.
- The active ordered cleanup backlog lives in `tmp/cleanup_plan.md`.

## Next tasks (ordered)

1. Treat `tmp/perf_plan.md` as the completed runtime perf execution record for items 1-11.
2. Continue the active cleanup execution lane from `tmp/cleanup_plan.md` in strict ROI order; items 1-3 are complete and item 4 is next.
3. Use `docs/plans/active/runtime_performance_exec_plan.md` to decide whether a new perf follow-up lane should be opened after the cleanup lane.
4. Keep handoff docs synchronized on future cleanup or perf milestones:
   update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and the active `tmp/*.md` backlog in the same cycle.
