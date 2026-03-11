# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-11T16:31:21Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Cleanup audit refresh for the post-cleanup codebase.
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` remain complete.
- The active cleanup backlog in `tmp/cleanup_plan.md` was refreshed on 2026-03-11 and has 15 pending items.
- Cleanup Phase 2 has not started; it is waiting for explicit user confirmation.

## Next tasks (ordered)

1. Present the exact ordered backlog from `tmp/cleanup_plan.md` and get explicit user confirmation before starting implementation.
2. If the user confirms, execute cleanup strictly in plan order starting at item 1.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized after each cleanup milestone.
4. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a separate perf follow-up lane is explicitly reopened.
