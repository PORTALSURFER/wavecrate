# Agent Memory

Last Updated: 2026-04-03T17:05:00+02:00
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
- I completed another one-shot bughunting pass on the current tree and landed three more focused fixes:
  - retained folder-delete restore now clears stale `last_played_at` metadata when the deleted snapshot says there was no playback history, with a regression test in `src/app/controller/library/source_folders/delete_recovery/retained_restore/tests.rs`
  - native `CommitFocusedBrowserRow` now stays a browser no-op when the browser still has focus but the previewed row was hidden by search/filtering, with coverage in `src/app_core/controller/tests/contextual_actions.rs`
  - waveform/browser automation snapshots now advertise the scroll and click-clear/play actions that the desktop GUI pack already drives, with parity coverage in `src/gui_test/runner/tests/action_parity.rs`
- I continued the ROI-ranked bug backlog and landed four more validated fixes:
  - file-op journal replay now verifies staged and target file identity before replaying metadata, so recovery preserves the current target and staged copy instead of overwriting a path that was reused before replay; coverage lives under `src/sample_sources/db/file_ops_journal/`
  - retained-delete startup recovery now auto-cleans stale `Deleted` journal rows when the staged folder was already purged and the original folder is still gone, instead of leaving a permanent inconsistent retained entry
  - retained folder-delete staging now avoids `staged_relative` values that are still reserved by old journal rows even when the on-disk staging folder is gone
  - previewing a browser row and then committing that same row now still applies commit-time focus history and similarity-refresh side effects, backed by `src/app/controller/tests/browser_actions/focus_navigation/commit_focus.rs`
- I completed another one-shot bughunting pass and landed one focused semantic-contract fix:
  - native waveform projection now keeps exposing the current sample label while audio is still loading, so map-focus and other async preview flows no longer leave `waveform.region` semantically blank before decode completes; coverage lives in `src/app_core/native_shell/tests/waveform.rs`
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` is green on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` is green again after rerunning the lane cleanly in a single cargo process with no orphaned compiler jobs.
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` are green after the file-op and retained-delete backlog fixes.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` now passes the root `app_core::actions` and `gui_test` phases after the latest automation parity fixes, but its final `vendor/radiant` smoke step is still blocked by older pane-migration/sidebar test compile failures inside `vendor/radiant`.
- The current `run_gui_contract` failure shifted back into the root crate before the `vendor/radiant` smoke step: `gui_test::packs::tests::contract_smoke_pack_runs_cleanly` fails because `map_point_focus` expects `waveform.region` to expose the focused sample label after `FocusMapSample`, while the native waveform projection previously only surfaced `ui.loaded_wav`.
- I patched `src/app_core/native_shell/waveform_projection.rs` and `src/app_core/native_shell.rs` to fall back to `ui.waveform.loading` for the projected waveform label, and I added a regression test in `src/app_core/native_shell/tests/waveform.rs`.
- Validation for this latest fix is not yet trustworthy in the current session: repeated Rust commands (`cargo check`, targeted `cargo test`, and `scripts/run_gui_contract.ps1`) abort with tool-level exit code `-1` after partial build output even when I serialize commands and isolate `CARGO_TARGET_DIR`.
- The next ranked bug-backlog items are:
  - rerun the root GUI contract lane once the local Cargo environment stops aborting with `-1` so the new waveform-loading label fix is explicitly validated
  - exact action-parity coverage is still missing for panel-wide node families beyond the current representative-node assertions
  - deciding whether to revive or replace the stale `vendor/radiant` test lanes that still block the final `run_gui_contract` smoke step once the root contract is green again
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Rerun `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` after the local Cargo session is stable enough to produce a reliable exit code, then run `scripts/ci_agent.ps1` before any push.
2. Commit the waveform-loading semantic-contract fix once a trustworthy validation pass exists; do not push until `ci_agent` is green again.
3. Continue with the next ranked remaining GUI-contract bug after the root lane is green, starting with broader action-parity coverage and then the stale `vendor/radiant` smoke step.
4. Keep recording each completed item back into `tmp/improvement_audit_plan.md`, `AGENTS.md`, `MEMORY.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md`.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (Phase 2 active; items 1 and 2 complete locally and validated)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

