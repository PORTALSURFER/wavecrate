# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-31T14:37:02+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-03-31.
- Phase 2 is active; items 1 and 2 are complete, and item 3 is next.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1 -SkipCi` is green on the current tree, and `tmp/cleanup_audit_hotspots.md` has been refreshed for the current audit.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Execute item 3 from `tmp/improvement_audit_plan.md`: split the migration-facing native action dispatch hubs in `app_core` into smaller surface-specific helpers with direct local tests.
2. Keep `tmp/improvement_audit_plan.md` current as the live execution record as the backlog advances.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the lane status changes.
