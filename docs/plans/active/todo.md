# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-31T11:34:48+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the completed Phase 2 execution record for the refreshed evidence-driven improvement audit of the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-03-31.
- All four plan items are complete in the working tree.
- `scripts/check_migration_boundary.ps1`, `scripts/check_file_size_budget.ps1 -All`, `scripts/check_quality_score_drift.ps1`, `scripts/run_gui_contract.ps1`, `scripts/run_agent_request.ps1`, and `scripts/ci_agent.ps1` are green again.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the user to choose the next lane.
2. Keep `tmp/improvement_audit_plan.md` as the completed execution record for this audit lane.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when a new lane starts.
