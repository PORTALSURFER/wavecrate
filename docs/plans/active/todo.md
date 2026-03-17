# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-17T13:45:00+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The refreshed evidence-driven improvement audit backlog was executed sequentially on 2026-03-17.
- `tmp/improvement_audit_plan.md` is the source of truth for the completed execution record.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Treat `tmp/improvement_audit_plan.md` as the completed execution record for the refreshed audit lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Start a new audit or follow-on implementation lane only after explicit user direction.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
