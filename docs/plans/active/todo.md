# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-08T03:55:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-10 in `tmp/perf_plan.md` are complete.
- The active ordered backlog lives in `tmp/perf_plan.md`.

## Next tasks (ordered)

1. Execute `tmp/perf_plan.md` item 11:
   borrow `ShellLayout` through hot native input handlers instead of cloning it repeatedly.
2. Run a full pass over remaining perf-plan validation and handoff docs after item 11 lands.
3. Keep handoff docs synchronized at each perf milestone:
   update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/perf_plan.md` in the same cycle.
