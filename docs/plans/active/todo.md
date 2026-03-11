# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-11T13:30:50Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` are complete.
- The refreshed cleanup backlog in `tmp/cleanup_plan.md` has 15 items; Phase 2 is in progress, items 1-7 are complete, and item 8 is next.

## Next tasks (ordered)

1. Continue the refreshed cleanup backlog in strict order from item 8.
2. Record item completions in `tmp/cleanup_plan.md` as each refactor commit lands.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized at the next milestone.
4. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a new perf follow-up lane is explicitly opened after cleanup.
