# Agent Memory

Last Updated: 2026-02-20T13:14:20Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am finalizing and shipping the Phase 5 stabilization batch for runtime
  projection and dirty-path behavior.
- In `src/app_core/native_shell.rs`, I hardened browser retained-cache lifecycle
  handling and cache-entry validation, and added targeted tests for revision
  invalidation, same-length selected-path updates, and stale cached-row
  refreshes.
- In `src/app_core/native_bridge.rs`, I added tests that lock waveform
  overlay-vs-view dirty classification semantics and refresh policy behavior.
- In `src/app/controller/state/runtime/derived_graph.rs`, I added test coverage
  for overlay dirty-reason propagation through descendants.
- In `docs/plans/active/runtime_performance_exec_plan.md`, I documented Phase 5
  stabilization milestones as complete.
- Full `bash scripts/ci_local.sh` is now green after the stabilization updates.

## Work Notes

- Pending commit/push: Phase 5 stabilization hardening/tests/docs on top of the
  prior projection and dirty-path performance work.
