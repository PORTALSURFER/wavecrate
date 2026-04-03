# Agent Memory

Last Updated: 2026-04-03T13:08:00+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 repo-wide ROI-ranked backlog for the current tree.
- Phase 2 is now active for the 2026-04-02 improvement audit backlog.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; that snapshot is the current supporting hotspot picture behind the new ranked plan.
- Items 1 and 2 from `tmp/improvement_audit_plan.md` are implemented, validated, committed, and pushed:
  - playback-age filter invalidation in `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` now rolls at the next relevant filter-boundary change instead of every second
  - the browser pipeline tests now cover the duplicate-cleanup, marked-only, text-query, similarity-query, week-rollover, and month-only rollover paths
- I completed a one-shot bughunting pass on the current tree and landed two focused fixes:
  - `commit_focused_browser_row()` now refuses to commit hidden stale browser focus when filters/search hide the previously focused sample, with a regression test in `src/app/controller/tests/browser_actions/focus_navigation/commit_focus.rs`
  - folder-row automation now advertises only row-scoped actions, and the root GUI contract lane covers that behavior through the new deterministic `sources` fixture plus action-parity assertions
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` is green on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` is green again after rerunning the lane cleanly in a single cargo process with no orphaned compiler jobs.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` now passes the root `app_core::actions` and `gui_test` phases, but its final `vendor/radiant` smoke step is still blocked by older pane-migration test compile failures inside `vendor/radiant`.
- The next ranked items are:
  - consolidating browser focus/selection ownership between `selection_ops.rs` and `focus_navigation.rs`
  - splitting the `vendor/radiant` hotkey catalog and the oversized `native_vello` gesture test hubs
  - deciding whether to revive or replace the stale `vendor/radiant` test lanes that still block the final `run_gui_contract` smoke step
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Continue with item 3 from `tmp/improvement_audit_plan.md` in ranked order.
2. Decide whether the stale `vendor/radiant` pane-migration test failures should be fixed now or explicitly deferred as a separate lane.
3. Keep recording each completed item back into `tmp/improvement_audit_plan.md`, `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md`.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (Phase 2 active; items 1 and 2 complete locally and validated)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
