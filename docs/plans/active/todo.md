# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-13T13:57:47Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Evidence-driven improvement audit planning.
- Phase 1 is complete on disk in `tmp/improvement_audit_plan.md`.
- No improvement-audit implementation work has started; explicit user confirmation is required before Phase 2 begins.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for explicit user confirmation before implementing anything from `tmp/improvement_audit_plan.md`.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. If Phase 2 is approved, execute `tmp/improvement_audit_plan.md` in ROI order and use `devcheck.ps1` plus `ci_quick.ps1` as the Windows validation gate.
4. After the active lane changes again, sync `AGENTS.md`, `MEMORY.md`, and this file.
