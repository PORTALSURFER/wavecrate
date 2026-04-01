# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-01T20:33:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-01.
- Phase 2 is active; item 1 is complete, item 2 is next, and execution is proceeding in backlog order.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` is green on the current tree, and `tmp/cleanup_audit_hotspots.md` has been refreshed for the current audit.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Execute item 2 from `tmp/improvement_audit_plan.md`: make GUI action-trace assertions distinguish handled behavior from mere dispatch attempts.
2. Continue with item 3 after item 2 unless item 2 exposes a new blocker that the plan needs to record.
3. Keep `tmp/improvement_audit_plan.md` current as the live backlog and execution record.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the lane status changes.
