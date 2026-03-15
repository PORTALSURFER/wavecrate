# Agent Memory

Last Updated: 2026-03-15T11:17:19Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is execution of the refreshed evidence-driven improvement audit backlog.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-15` from the live `6c9dc2d8` tree and now doubles as the execution record for Phase 2.
- Phase 2 is in progress.
- Item 1 is complete: `scripts/ci_quick.ps1` now runs `scripts/run_gui_contract.ps1`, and `docs/gui_test_platform.md` is aligned with that default quick gate.
- Item 1 implementation commit is `2fddca31`.
- Item 2 is complete: browser multi-selection is now path-authoritative, with derived selected indices cached lazily for index-driven projection and controller code.
- Item 2 implementation commit is `7338908a`.
- Item 3 is complete: `browser_search` is now split into focused `cache`, `dispatch_policy`, and `mutations` modules behind the existing browser facade.
- Item 3 implementation commit is `cddf369d`.
- Item 4 is complete: `browser_lists` is now split so rebuild/prune orchestration, projection application, and lookup-map maintenance live in separate modules.
- Item 4 implementation commit is `ba52b318`.
- Item 5 is complete: the folder-browser tree module is now split across retained model, scan orchestration, and projection/filter helpers.
- Item 5 implementation commit is `cb561557`.
- Item 6 is complete: the hotkey registry now has direct invariant tests for unique ids, same-scope gesture conflicts, and global-vs-focus lookup separation.
- Item 6 implementation commit is `3dfca2e0`.
- Item 7 is complete: `wav_sanitize` now documents its narrow repair scope, enforces logical `Read + Seek` behavior after rewinding into the sanitized header, and keeps direct seek coverage in `src/wav_sanitize/tests.rs`.
- Item 7 implementation commit is `06d94dc6`.
- The highest-leverage current findings are:
  - Deferred undo/file-op flows remain under-defined relative to the trust they carry.
  - The semantic automation tree still does not cover the remaining browser action-strip buttons and similar micro-controls.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Implement item 8 from `tmp/improvement_audit_plan.md`: add targeted deferred undo/redo file-flow coverage and separate generic undo primitives from controller/file glue where that reduces coupling.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned while Phase 2 advances.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Treat `scripts/ci_local.ps1` green as the current expected Windows local parity baseline.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

