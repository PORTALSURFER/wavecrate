# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-10T18:57:09Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` are complete.
- The active ordered cleanup backlog lives in `tmp/cleanup_plan.md`; items 1-15 are complete, item 16 is next, and execution is paused pending explicit user reconfirmation.

## Next tasks (ordered)

1. Present the refreshed `tmp/cleanup_plan.md` backlog and get explicit confirmation before resuming cleanup at item 16.
2. If the user confirms, continue cleanup item 16 from `tmp/cleanup_plan.md` in strict ROI order.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized on the next cleanup or perf milestone.
4. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a new perf follow-up lane is opened after cleanup.
