# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-12T20:03:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Cleanup audit refresh for the post-cleanup codebase.
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` remain complete.
- The active cleanup backlog in `tmp/cleanup_plan.md` is now in Phase 2 execution.
- Cleanup items 1-7 are complete, and item 8 is next.

## Next tasks (ordered)

1. Continue cleanup strictly in plan order at item 8.
2. After each cleanup milestone, rerun validation and sync `tmp/cleanup_plan.md`, `AGENTS.md`, `MEMORY.md`, and this file.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized after each cleanup milestone.
4. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a separate perf follow-up lane is explicitly reopened.
