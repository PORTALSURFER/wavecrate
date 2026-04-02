# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T00:02:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the repo-wide improvement backlog rebuilt on 2026-04-02.
- Phase 2 is active and items 1-2 from `tmp/improvement_audit_plan.md` are implemented locally and validated.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`, `scripts/check_file_size_budget.ps1 -All`, and `scripts/check_quality_score_drift.ps1` are green on the live tree.
- `tmp/cleanup_audit_hotspots.md` was refreshed during this audit and is the current supporting hotspot snapshot.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Commit and push the completed item-2 browser-pipeline coverage expansion plus its plan updates.
2. Continue with item 3 from `tmp/improvement_audit_plan.md` after the item-2 commit lands.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
