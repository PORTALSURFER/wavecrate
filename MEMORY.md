# Agent Memory

Last Updated: 2026-04-02T22:09:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 ROI-ranked backlog and execution record for the current tree.
- Phase 2 is complete. Item 1 (`restore the app_core migration boundary for folder-pane state types`) is complete in commit `222eda75` (`fix(app-core): restore folder-pane migration boundary`), item 2 (`split drop-target transfer orchestration`) is complete in commit `ffb70a38` (`refactor(drag-drop): split drop-target transfer planning`), item 3 (`split vendor/radiant folder chrome hit-testing`) is complete in vendor commit `75b6d980` plus superproject commit `ad3a487a` (`refactor(radiant): split folder chrome hit testing`), item 4 (`split waveform_nav_render.rs`) is complete in commit `41cee5b5` (`refactor(tests): split waveform nav render coverage`), item 5 (`split browser_core/marks.rs`) is complete in commit `48a52f50` (`refactor(tests): split browser mark coverage`), item 6 (`strengthen automation action-id parity checks`) is complete in commit `6b0f889d` with follow-up isolation commit `33b7f493`, and item 7 (`refresh stale long-form migration/quality status docs`) is complete in commit `d07f6079`; all are pushed to `origin/next`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` now passes after the item 1 alias repair.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` also passes after item 1.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` now passes.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` is still partially blocked by pre-existing unrelated dirty-`vendor/radiant` test compile failures after the catalog and `gui_test::` slices pass, so the item 6 parity change is validated by those passing slices plus `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` now passes after the long-form status docs were refreshed.
- `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1` also pass after the doc refresh.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; that snapshot records the audit-start hotspot picture, while the current live full-scan file-size budget is now green.
- `docs/QUALITY_SCORE.md` and `docs/gui_migration_parity.md` now match the live guardrail state for this lane.
- The live tree still passes these other guardrails:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_legacy_app_coupling.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_public_docs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_rust_no_todos.ps1`
- The ranked improvement backlog is complete through item 7.
- The new audit also records three open questions:
  - long-term action-id ownership across `app_core` and `vendor/radiant`
  - how far future lanes should push `app_core` away from legacy `AppController` layout dependence
  - which artifacts future agents should treat as authoritative when long-form status docs lag the live guardrails
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the completed execution record for the 2026-04-02 improvement audit lane.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned if a new lane starts.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (refreshed on 2026-04-02 after live drift recheck; Phase 2 complete with items 1-7 done)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
