# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-02T18:58:11+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-02 after a live drift recheck.
- Phase 2 is in progress. Item 1 completed in commit `222eda75`, item 2 completed in commit `ffb70a38`, and the migration-boundary preflight remains green.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is currently red on `src/app/controller/tests/browser_core/marks.rs`, `src/app/controller/tests/waveform_nav_render.rs`, and `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs`.
- `docs/QUALITY_SCORE.md` and `docs/gui_migration_parity.md` currently lag the live guardrail state and should be treated as stale until the backlog reaches the documentation refresh item.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Implement item 3 from `tmp/improvement_audit_plan.md`: split `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs`.
2. Keep `tmp/improvement_audit_plan.md` updated after each completed item, including validation notes and commit hashes.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/index.md` synchronized with this Phase 2 execution state.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
