# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-15T11:51:34Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The improvement audit Phase 2 lane is complete.
- `tmp/improvement_audit_plan.md` is the source of truth and finished execution record for that lane.
- Items 1-9 are complete.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the next user-directed lane; keep `tmp/improvement_audit_plan.md` as the completed execution record.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
