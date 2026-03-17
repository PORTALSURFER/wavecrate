# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-17T11:27:56+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- A refreshed evidence-driven improvement audit was completed against the live tree on 2026-03-17.
- `tmp/improvement_audit_plan.md` is the source of truth for the current Phase 1 backlog.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for explicit user confirmation before implementing any ranked item from `tmp/improvement_audit_plan.md`.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Treat stale hotspot/scorecard docs as backlog work, not as current truth, until the new backlog is executed.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
