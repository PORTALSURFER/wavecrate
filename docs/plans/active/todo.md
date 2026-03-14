# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-14T10:01:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit execution, Phase 2 active.
- `tmp/improvement_audit_plan.md` remains the source of truth for ordered execution and status tracking.
- Backlog items 1-5 are complete in the working tree; item 5 still needs its focused commit/push before the lane proceeds to item 6.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Commit and push improvement audit backlog item 5 from `tmp/improvement_audit_plan.md`.
2. Start improvement audit backlog item 6 from `tmp/improvement_audit_plan.md`.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. After each completed audit item, sync `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md`.
