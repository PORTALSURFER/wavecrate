# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-14T12:25:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit Phase 2 execution is active.
- `tmp/improvement_audit_plan.md` is the source of truth for the refreshed backlog and execution record.
- Backlog item 1 is complete; item 2 is next.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Continue executing `tmp/improvement_audit_plan.md` from item 2.
2. Resolve or document the `vendor/radiant` baseline failure exposed by `scripts/ci_local.ps1`.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
