# Agent Memory

Last Updated: 2026-04-02T16:58:11Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 ROI-ranked backlog for the current tree.
- Phase 1 is complete and Phase 2 has not started. I must not implement any ranked item until the user explicitly confirms.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` currently fails during preflight because `scripts/check_migration_boundary.ps1` finds direct `crate::app::` references outside `src/app_core/app_api.rs`.
- The current migration-boundary violations are limited to:
  - `src/app_core/controller/browser_actions/browser.rs`
  - `src/app_core/controller/browser_actions/folders.rs`
  - `src/app_core/controller/waveform_actions/selection.rs`
  - `src/app_core/native_shell/sources_projection.rs`
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails on three non-allowlisted files:
  - `src/app/controller/tests/browser_core/marks.rs`
  - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs`
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs`
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` currently downgrades the quality score to `3` because the file-size budget is red.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; the live full scan shows `11` over-budget files total, with `8` documented allowlist exceptions and the `3` current non-allowlisted regressions above.
- The live tree still passes these other guardrails:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_legacy_app_coupling.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_public_docs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_no_todos.ps1`
- The new backlog ranks five items:
  - restore the `app_core` migration boundary for folder-pane state types
  - split `drop_targets.rs`
  - split `vendor/radiant` folder hit-testing
  - split `browser_core/marks.rs`
  - strengthen automation action-id parity tests
- The new audit also records two open questions: long-term action-id ownership across `app_core` and `vendor/radiant`, and how far future lanes should push `app_core` away from legacy `AppController` layout dependence.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the current Phase 1 source of truth until the user explicitly confirms implementation.
2. Do not implement any ranked item until explicit user confirmation arrives.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with the new Phase 1 audit state.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (refreshed on 2026-04-02; Phase 1 complete; awaiting explicit Phase 2 confirmation)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
