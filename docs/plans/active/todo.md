# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-14T10:47:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit execution lane complete.
- `tmp/improvement_audit_plan.md` remains the source of truth for the completed execution record.
- Backlog items 1-10 are complete and pushed on `next`.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the next user-directed lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. If follow-up audit work is requested, resume from `tmp/improvement_audit_plan.md` rather than rebuilding the backlog from scratch.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
