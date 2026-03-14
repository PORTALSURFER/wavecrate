# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-14T18:03:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit Phase 2 safe work is complete.
- `tmp/improvement_audit_plan.md` is the source of truth for the refreshed backlog and execution record.
- Backlog items 1-8 are complete.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the next user-directed lane; do not reopen the completed improvement-audit plan unless follow-up audit work is explicitly requested.
2. Keep `tmp/improvement_audit_plan.md` as the completed execution record for the latest audit lane.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
