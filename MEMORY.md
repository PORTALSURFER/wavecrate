# Agent Memory

Last Updated: 2026-03-06T14:51:46Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have split `src/app/controller/library/background_jobs/polling.rs` into focused routing/helper/handler modules under `src/app/controller/library/background_jobs/polling/`.
- I added behavior tests covering stale folder-scan drops, stale/current browser-search routing, and file-op progress/finish state transitions.
- Full local CI is green in native PowerShell (`scripts/ci_local.ps1`), including fmt, clippy, rustdoc, 871 tests, and perf guard.
- Cleanup Phase 2 is active, and `tmp/cleanup_plan.md` item 3 is completed in commit `4a4c1098`.

## Immediate Next Actions

1. Continue the ordered cleanup backlog at item 4 (`src/app/controller/library/wavs/browser_actions.rs`) if the user keeps the cleanup lane active.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` aligned on the next cleanup milestone.
3. Resume `tmp/perf_plan.md` only after the cleanup request is complete or redirected.

## Work Notes

- Background-job polling cleanup now lives under `src/app/controller/library/background_jobs/polling/` with focused files for audio, routing, library handlers, runtime handlers, and tests.
- Active cleanup backlog: `tmp/cleanup_plan.md`.
- Runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
