# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-02T00:34:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-01.
- Phase 2 is active; items 1, 2, and 3 are complete, item 4 is next, and execution is proceeding in backlog order.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` is green on the current tree, and `tmp/cleanup_audit_hotspots.md` has been refreshed for the current audit.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Execute item 4 from `tmp/improvement_audit_plan.md`: finish the `app_core` dispatch-hub split so migration-facing routing depends on narrower controller seams.
2. Continue with item 5 after item 4 unless item 4 exposes a new blocker that the plan needs to record.
3. Keep `tmp/improvement_audit_plan.md` current as the live backlog and execution record.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the lane status changes.
