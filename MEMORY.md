# Agent Memory

Last Updated: 2026-04-04T16:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The user confirmed Phase 2 of the rebuilt runtime performance lane, and execution is now in progress.
- The current workspace is dirty with unrelated user edits, including `docs/README.md`, `docs/plans/index.md`, and multiple controller files outside this performance lane; I must not overwrite them.
- `tmp/perf_plan.md` is now the active Phase 2 execution record for the rebuilt runtime-performance lane.
- Item 1 is complete in `vendor/radiant` commit `e5c91739` (`perf(app): retain projected rows across model clones`) and root commit `3c91fbef` (`perf(app_core): retain projected row collections`).
- Item 1 now keeps browser, source, and folder row collections behind retained shared vectors and stores browser row text in shared `Arc<str>` buffers so top-level app-model clones no longer copy those payloads on browser/map/static churn.
- The latest perf-guard snapshot after item 1 reports `browser_filter_churn_latency = 2132us` p95 and `projection_stage = 2098us` p95 in `target/perf/bench.json`, down from the Phase 1 audit snapshot of `2396us` and `2342us`.
- Item 2, latest-only waveform render plus async transient work, is next in strict ROI order.
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked unless the user explicitly reopens those lanes.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 2 from `tmp/perf_plan.md` next, then rerun relevant validation, update the plan, commit, and push.
2. Keep the runtime-performance work in strict ROI order unless the user redirects the lane.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` parked unless the user explicitly reopens those lanes.
4. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/perf_plan.md` (Phase 2 in progress on 2026-04-04; item 1 complete)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked improvement backlog: `tmp/improvement_audit_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`



