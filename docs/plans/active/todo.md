# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T00:09:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-02 after a live drift recheck.
- Phase 2 is complete. Item 1 completed in commit `222eda75`, item 2 in `ffb70a38`, item 3 in vendor `75b6d980` plus superproject `ad3a487a`, item 4 in `41cee5b5`, item 5 in `48a52f50`, item 6 in `6b0f889d` plus follow-up isolation commit `33b7f493`, and item 7 in `d07f6079`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is now green again.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`, `scripts/check_docs_index.ps1`, and `scripts/check_markdown_links.ps1` are now green after the doc refresh.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` is still partially blocked by unrelated dirty-`vendor/radiant` test compile failures after the catalog and `gui_test::` slices pass.
- `docs/QUALITY_SCORE.md` and `docs/gui_migration_parity.md` now match the live guardrail state for this lane.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Treat `tmp/improvement_audit_plan.md` as the completed execution record for this lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Start a new active TODO only when the user confirms a new lane.
