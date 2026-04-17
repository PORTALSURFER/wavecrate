# Agent Memory

Last Updated: 2026-04-17T19:57:30+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The current workspace is dirty with unrelated user edits; I must not overwrite them while executing the perf lane.
- `tmp/perf_plan.md` is now the active source of truth for the rebuilt 2026-04-17 follow-up runtime performance backlog.
- I completed Phase 1 only on 2026-04-17. I audited the current tree with fresh local measurements plus read-only subaudits and I am waiting for explicit user approval before Phase 2 implementation.
- Fresh evidence captured on 2026-04-17:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`
  - `target/perf/bench.json` with `controller_app_model_projection.p95_us = 2826`, `retained_app_model_projection_p95_us = 5`, `browser_filter_churn_latency.p95_us = 38`, `browser_query_churn_latency.p95_us = 65`, `wheel_latency.p95_us = 419`, `waveform_interaction_latency.p95_us = 108`, and `feature_blob_decode.total_elapsed_ms = 5329` for `320000` blobs.
  - `SEMPAL_PERF_GUARD_STARTUP_PROFILE=1 powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`
  - `target/perf/bench..startup_summary.json` with `first_present_ms = 1632.444`, `surface_ready_ms = 904.170`, `renderer_ready_ms = 1205.318`, and `deferred_model_refresh_ms = 371.057`.
  - Waveform preview A/B: `target/perf/bench_default_preview.json` vs `target/perf/bench_preview_off.json` shows disabling immediate waveform preview regresses `waveform_interaction_latency` (`p95 158us -> 1304us`, `p99 215us -> 13518us`).
- The current ordered follow-up backlog in `tmp/perf_plan.md` focuses on:
  - collapsing browser search-worker reload/revision refresh into one retained pass
  - narrowing sync browser-pipeline invalidation for metadata-only edits
  - splitting the remaining controller fallback prep lane beyond `BrowserRetainedPull`
  - removing duplicate text shaping in active text fields
  - replacing entry-sized similarity scratch with sparse/windowed lookup
  - tightening native text/frame-text caches
  - moving lightweight feature metrics off the full feature-blob decode hot path
  - retuning hidden-startup reveal policy with measurement and manual visual review
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/perf_plan.md` as the active Phase 1 runtime-performance source of truth.
2. Wait for explicit user approval before starting Phase 2 implementation work.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.
4. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (Phase 1 rebuilt on 2026-04-17; awaiting explicit Phase 2 approval)
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`


