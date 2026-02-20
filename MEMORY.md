# Agent Memory

Last Updated: 2026-02-20T12:23:12Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the next runtime performance milestone focused on hover and
  wheel responsiveness hot paths.
- In `vendor/radiant/src/gui_runtime/native_vello.rs`, I replaced clone-heavy
  state/motion overlay fingerprint cache keys with compact deterministic
  signatures so overlay skip checks avoid repeated model cloning.
- In `src/app_core/native_bridge.rs`, I moved wheel focus dirty/invalidation
  handling to flush-time and now skip projection-cache invalidation when queued
  focus deltas produce no effective model-key change.
- I updated the corresponding bridge test to assert the no-op focus path keeps
  the projection cache key.
- `bash scripts/ci_local.sh` is green after these changes; perf guard reports
  warn-only drift and remains non-failing.

## Work Notes

- Pending commits (not yet pushed): native-vello overlay fingerprint
  signature optimization and native bridge no-op wheel focus invalidation skip.
