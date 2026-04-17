# Agent Memory

Last Updated: 2026-04-17T17:48:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The runtime performance lane is in Phase 2 execution for the current live tree.
- The current workspace is dirty with unrelated user edits; I must not overwrite them while executing the perf lane.
- `tmp/perf_plan.md` is the live source of truth for the rebuilt 2026-04-17 runtime performance backlog.
- I rebuilt the backlog from a fresh local `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` run on 2026-04-17 and started Phase 2 after explicit user confirmation.
- Items 1-3 are complete in `tmp/perf_plan.md` (`cec627fd`, `perf(native-bridge): add retained browser prep lane`; `547a9c9b`, `perf(browser): retain filter and folder stage indexes`; `140a8640`, `perf(bench): measure retained runtime in guard`; vendor `174aa295`, `perf(native-vello): emit eager startup summaries`), and item 4 is now next in strict ROI order.
- The latest default perf-guard artifact completed without warnings and now headlines the shipped retained bridge path: `app_model_projection = 5us` p95, `controller_app_model_projection = 2708us` p95, `retained_app_model_projection_p95_us = 5`, `hover_latency = 261us` p95, `wheel_latency = 508us` p95, `browser_filter_churn_latency = 46us` p95, and `browser_query_churn_latency = 53us` p95.
- The Windows PowerShell perf guard now captures startup summaries; the reduced startup smoke run recorded `first_present_ms = 2377.742` and `deferred_model_refresh_ms = 0.000` while emitting recommended threshold locks.
- The rebuilt backlog themes are:
  - broad `prepare_native_frame(false)` maintenance before retained pulls
  - sync browser filter/folder rescans in the controller projection path
  - perf guard misalignment with the shipped retained runtime and missing Windows startup capture
  - metadata-only browser refreshes that still load full search rows
  - similarity duplicate checks that decode whole feature blobs just to read RMS
  - vendor browser-row cache invalidation that is broader than needed during real scene rebuilds
  - startup reveal policy that still waits for an eager hidden full-scene launch
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute `tmp/perf_plan.md` strictly in listed ROI order, one item at a time, starting with item 4 next.
2. After each implemented item in Phase 2, update `tmp/perf_plan.md`, run validation, commit, and push.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.
4. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (Phase 2 in progress on 2026-04-17; items 1-3 complete, item 4 next)
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


