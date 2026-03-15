# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-15T12:42:32Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- A fresh improvement audit Phase 1 lane is complete.
- `tmp/improvement_audit_plan.md` is the source of truth for the refreshed backlog.
- The backlog is awaiting explicit implementation approval.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for explicit user confirmation before implementing any item from `tmp/improvement_audit_plan.md`.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized when the active lane changes.
