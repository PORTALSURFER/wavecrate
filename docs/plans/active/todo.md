# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T17:05:00+02:00
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
- A follow-up one-shot bughunting pass landed:
  - retained restore metadata replay now clears stale `last_played_at` timestamps when the deleted snapshot has none
  - native browser Enter no longer toggles transport when the focused row is hidden by search/filtering
  - waveform/browser automation nodes now advertise the click/play, clear-selections, and browser-scroll actions that the desktop GUI pack already drives
- The ROI-ranked bug backlog pass continued and landed:
  - file-op journal replay now defers on staged/target identity mismatches instead of overwriting a reused target path
  - retained-delete recovery now self-heals stale `Deleted` rows when the staged folder was already purged
  - retained delete staging now skips `staged_relative` values that are still reserved by stale journal rows
  - previewing a browser row and then committing that same row now still applies commit-time history and similarity side effects
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` are green on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passes the root catalog and `gui_test` phases, but its final `vendor/radiant` smoke step is still blocked by stale pane-migration/sidebar test compile failures in `vendor/radiant`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`, `scripts/check_file_size_budget.ps1 -All`, and `scripts/check_quality_score_drift.ps1` are green on the live tree.
- `tmp/cleanup_audit_hotspots.md` was refreshed during this audit and is the current supporting hotspot snapshot.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Commit and push the validated bug-backlog fixes on `next`.
2. Continue with the next remaining ranked bug item, starting with the stale-anchor behavior in `focus_browser_list()`.
3. Decide whether the stale `vendor/radiant` pane-migration test failures should be fixed now or explicitly deferred into their own lane.
4. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO synchronized after each completed item.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
