# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-06T14:51:46Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Cleanup architecture guardrails documented in `docs/plans/active/cleanup_architecture_note.md`.
- Cleanup Phase 2 item 3 completed on 2026-03-06 UTC in commit `4a4c1098`.

## Next tasks (ordered)

1. Execute `tmp/cleanup_plan.md` item 4:
   decompose `src/app/controller/library/wavs/browser_actions.rs` by responsibility and expand focused browser-behavior tests.
2. Execute `tmp/cleanup_plan.md` item 5:
   split folder-delete recovery into journal/recovery/controller-apply modules and add recovery-matrix coverage.
3. Execute `tmp/cleanup_plan.md` item 6:
   isolate source-move DB/fs stage transitions and centralize error/report policy.
4. Keep handoff docs synchronized at each cleanup milestone:
   update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` in the same cycle.
