# Agent Memory

Last Updated: 2026-04-02T20:32:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 ROI-ranked backlog and execution record for the current tree.
- Phase 2 is in progress. Item 1 (`restore the app_core migration boundary for folder-pane state types`) is complete in commit `222eda75` (`fix(app-core): restore folder-pane migration boundary`) and item 2 (`split drop-target transfer orchestration`) is complete in commit `ffb70a38` (`refactor(drag-drop): split drop-target transfer planning`); both are already pushed to `origin/next`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` now passes after the item 1 alias repair.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` also passes after item 1.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` currently fails on three non-allowlisted files:
  - `src/app/controller/tests/browser_core/marks.rs`
  - `src/app/controller/tests/waveform_nav_render.rs`
  - `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome/folders.rs`
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` currently downgrades the quality score to `3` because the file-size budget is red.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; the live full scan shows `12` over-budget files total, with `8` documented allowlist exceptions and the `4` current non-allowlisted regressions above.
- `docs/QUALITY_SCORE.md` currently overstates the live guardrail state by saying the full-scan file-size budget is green.
- `docs/gui_migration_parity.md` currently names older playback-age migration blockers instead of the live folder-pane `app_core` violations.
- The live tree still passes these other guardrails:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_legacy_app_coupling.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_public_docs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_no_todos.ps1`
- The remaining backlog ranks five pending items:
  - split `vendor/radiant` folder hit-testing
  - split `waveform_nav_render.rs`
  - split `browser_core/marks.rs`
  - strengthen automation action-id parity tests
  - refresh stale long-form migration/quality status docs
- The new audit also records three open questions:
  - long-term action-id ownership across `app_core` and `vendor/radiant`
  - how far future lanes should push `app_core` away from legacy `AppController` layout dependence
  - which artifacts future agents should treat as authoritative when long-form status docs lag the live guardrails
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the current Phase 2 source of truth and continue with item 3 unless a new blocker appears.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with the live Phase 2 state.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (refreshed on 2026-04-02 after live drift recheck; Phase 2 in progress; items 1-2 complete in `222eda75` and `ffb70a38`)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
