# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-02T23:37:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-02 after a live drift recheck.
- Phase 2 is in progress. Item 1 completed in commit `222eda75`, item 2 completed in commit `ffb70a38`, item 3 completed in vendor commit `75b6d980` plus superproject commit `ad3a487a`, item 4 completed in commit `41cee5b5`, item 5 completed in commit `48a52f50`, item 6 completed in commit `6b0f889d`, and the migration-boundary preflight remains green.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is now green again.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` is still red because `docs/QUALITY_SCORE.md` understates the now-healthy guardrail score.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` is still partially blocked by unrelated dirty-`vendor/radiant` test compile failures after the catalog and `gui_test::` slices pass.
- `docs/QUALITY_SCORE.md` and `docs/gui_migration_parity.md` currently lag the live guardrail state and should be treated as stale until the backlog reaches the documentation refresh item.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Implement item 7 from `tmp/improvement_audit_plan.md`: refresh the stale long-form migration and quality status docs.
2. Keep `tmp/improvement_audit_plan.md` updated after each completed item, including validation notes and commit hashes.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/index.md` synchronized with this Phase 2 execution state.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
