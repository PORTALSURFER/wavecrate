# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-15T17:05:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit Phase 2 execution is in progress.
- `tmp/improvement_audit_plan.md` is the source of truth for the backlog and execution record.
- Items 1-3 are complete; item 4 is next in strict ROI order.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Implement item 4 from `tmp/improvement_audit_plan.md`: split `src/app/controller/library/wavs/browser_lists.rs` so browser projection refresh and lookup-map maintenance stop sharing one file.
2. Continue executing items strictly in plan order unless a documented blocker forces a deviation.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
