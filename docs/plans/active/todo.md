# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-12T11:39:30Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Cleanup audit refresh for the post-cleanup codebase.
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` remain complete.
- The active cleanup backlog in `tmp/cleanup_plan.md` was rebuilt from the current codebase on `2026-03-12`.
- Phase 1 is complete and Phase 2 must not begin until the user explicitly confirms the ordered backlog.

## Next tasks (ordered)

1. Review `tmp/cleanup_plan.md` with the user and wait for explicit Phase 2 confirmation.
2. If approved, execute cleanup strictly in plan order and sync `tmp/cleanup_plan.md`, `AGENTS.md`, `MEMORY.md`, and this file after each item.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized after each cleanup milestone.
4. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a separate perf follow-up lane is explicitly reopened.
