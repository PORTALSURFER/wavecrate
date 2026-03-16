# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-16T15:45:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The refreshed improvement-audit lane is complete.
- `tmp/improvement_audit_plan.md` is the source of truth for the completed ROI-ranked backlog and execution record.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the next user-directed lane before reopening `tmp/improvement_audit_plan.md`.
2. Keep `tmp/improvement_audit_plan.md` as the completed audit execution record.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
