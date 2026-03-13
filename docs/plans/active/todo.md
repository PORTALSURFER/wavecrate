# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-13T19:10:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Improvement audit execution handoff.
- `tmp/improvement_audit_plan.md` is complete through item 14 and now records the finished execution lane from `2026-03-13`.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the user to choose the next lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. If the user asks for full CI parity, note that the previous file-size-budget blocker is gone and the remaining blocker is the pre-existing `scripts/check_migration_boundary.ps1` termination behavior.
4. After the active lane changes again, sync `AGENTS.md`, `MEMORY.md`, and this file.
