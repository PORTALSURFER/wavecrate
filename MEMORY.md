# Agent Memory

Last Updated: 2026-03-31T11:34:48+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane has completed Phase 2 of a refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth and was regenerated on `2026-03-31`.
- The workspace is not fully clean; several unrelated user-dirty files remain outside the audit lane, including `tools/gui-test-cli/src/main.rs`, multiple root-side controller files, and dirty `vendor/radiant/**` files outside the staged audit split.
- The previous 2026-03-30/31 execution record at `tmp/improvement_audit_plan.md` has been replaced with a fresh Phase 1 backlog for the current observed tree.
- All four items from `tmp/improvement_audit_plan.md` are complete in the working tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` is green again after routing the browser playback-age helpers back through `app_core::state`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is green again after splitting the three active non-allowlisted hotspots into focused submodules.
- Focused validation for the completed work is green:
  - `cargo test -p sempal --lib app_core::controller::tests::dispatch -- --test-threads=1`
  - `cargo test -p radiant --lib browser_rows -- --test-threads=1`
- `vendor/radiant/src/gui/native_shell/state/tests/mod.rs` needed one safe prerequisite fix during validation because `CachedBrowserRow` now requires `playback_age_bucket`.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` is green again after the doc/status sync.
- `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/check_codeowners_coverage.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/check_app_core_dependency_boundary.ps1` all pass on the current tree.
- The broader validation lane is green:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- `tmp/improvement_audit_plan.md` is now the completed execution record for this lane.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for the user to choose the next lane.
2. Keep `tmp/improvement_audit_plan.md` honest as the completed execution record for this lane.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned when a new lane starts.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-31; Phase 2 completed and retained as the execution record)
- Current broader hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`




