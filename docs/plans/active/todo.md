# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-02T18:58:11+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is the refreshed evidence-driven improvement audit backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-04-02.
- Phase 1 is complete; Phase 2 has not started and must wait for explicit user confirmation.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` is currently red because `scripts/check_migration_boundary.ps1` fails on new direct `crate::app::` references inside `src/app_core`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is currently red on `src/app/controller/tests/browser_core/marks.rs`, `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs`, and `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs`.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Treat `tmp/improvement_audit_plan.md` as the current Phase 1 source of truth until the user explicitly confirms sequential implementation.
2. Do not implement the ranked items until explicit user confirmation arrives.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/index.md` synchronized with this Phase 1 audit state.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
