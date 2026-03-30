# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-30T13:30:46+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit of the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-03-30.
- Item 1 is complete; the live `app_core` migration-boundary failure at `HEAD` is fixed.
- Item 3 is complete.
- Item 8 burned down the clean root-side file-size debt and left only blocked or unrelated-dirty budget failures.
- Item 2 is still the next ranked unresolved item, and it remains clarification-gated.
- The remaining live file-size budget failures are limited to dirty `src/app_core/controller/tests/browser_sources.rs`, clarification-gated `src/gui_test/runner.rs`, and dirty `vendor/radiant/**` files.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for user clarification on item 2, item 5, or item 6 before resuming the remaining blocked audit backlog, or wait for direction on whether the dirty `src/app_core/controller/tests/browser_sources.rs` / `vendor/radiant/**` scope may be edited.
2. Keep `tmp/improvement_audit_plan.md` updated as the live execution record for the blocked remainder of item 8.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the paused-state blockers change.
