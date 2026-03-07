# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-08T01:05:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Perf Phase 2 items 1-8 in `tmp/perf_plan.md` are complete.
- The active ordered backlog lives in `tmp/perf_plan.md`.

## Next tasks (ordered)

1. Execute `tmp/perf_plan.md` item 9:
   retain map point identity buffers and apply selected/focused state as overlays.
2. Execute `tmp/perf_plan.md` item 10:
   replace proxy segment p95 attribution with true per-segment measured timings.
3. Execute `tmp/perf_plan.md` item 11:
   borrow `ShellLayout` through hot native input handlers instead of cloning it repeatedly.
4. Keep handoff docs synchronized at each perf milestone:
   update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/perf_plan.md` in the same cycle.
