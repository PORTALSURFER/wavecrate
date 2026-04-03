# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T13:08:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the repo-wide improvement backlog rebuilt on 2026-04-02.
- Phase 2 is active and items 1-2 from `tmp/improvement_audit_plan.md` are implemented locally and validated.
- A one-shot bughunting pass landed the hidden-stale browser focus fix and a folder-row automation contract fix backed by the new deterministic `sources` GUI fixture.
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` are green on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passes the root catalog and `gui_test` phases, but its final `vendor/radiant` smoke step is still blocked by stale pane-migration test compile failures in `vendor/radiant`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`, `scripts/check_file_size_budget.ps1 -All`, and `scripts/check_quality_score_drift.ps1` are green on the live tree.
- `tmp/cleanup_audit_hotspots.md` was refreshed during this audit and is the current supporting hotspot snapshot.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Continue with item 3 from `tmp/improvement_audit_plan.md` in ranked order.
2. Decide whether the stale `vendor/radiant` pane-migration test failures should be fixed now or explicitly deferred into their own lane.
3. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO synchronized after each completed item.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
