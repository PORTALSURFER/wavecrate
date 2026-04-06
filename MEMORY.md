# Agent Memory

Last Updated: 2026-04-06T12:38:14Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The runtime performance lane is in Phase 2 execution for the current live tree.
- The current workspace is dirty with unrelated user edits; I must not overwrite them while executing the perf lane.
- `tmp/perf_plan.md` is the live source of truth for the rebuilt 2026-04-05 runtime performance backlog.
- Items 1-3 are complete in `tmp/perf_plan.md` (`bd2b6a57`, `perf(similarity): cache loaded source snapshots`; `d27d9adc`, `perf(browser): retain search filter stages`; `e5d91fe3`, `perf(search): apply metadata deltas in place`), and item 4 is now next in strict ROI order.
- The strongest remaining measured hotspots in `target/perf/bench.json` are `hover_latency = 5138us` p95, `wheel_latency = 5106us` p95, `app_model_projection = 4115us` p95, `interactive_projection = 4008us` p95, and `browser_filter_churn_latency = 2872us` p95.
- The current perf artifact shows those headline browser hotspots are still projection-stage dominated (`hover = 4484us`, `wheel = 4336us`, `filter churn = 2810us` p95 projection stage), but the retained bridge diagnostic is now only `retained_app_model_projection_p95_us = 8`.
- Important caveat: the current perf guard still measures the controller-mode `project_native_app_model` path for most GUI scenarios, while the real native runtime in `main` uses `GuiFixtureBridge` and retained `SempalNativeBridge`; the Windows PowerShell perf guard also still lacks startup capture.
- The top backlog themes are:
  - loaded-similarity full-source embedding and feature scans
  - full-source browser filter/folder/similarity rebuilds
  - revision-wide browser search metadata refreshes
  - broad `prepare_native_frame(false)` maintenance before retained pulls
  - whole-window vendor browser-row cache invalidation
  - startup measurement gaps and eager hidden full-scene startup
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

- Active audit plan: `tmp/perf_plan.md` (Phase 2 in progress on 2026-04-06; items 1-3 complete, item 4 next)
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

