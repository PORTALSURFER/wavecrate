# Agent Memory

Last Updated: 2026-02-20T21:07:18Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am planning and implementing the next highest-ROI responsiveness milestone
  for sempal/radiant.
- In `vendor/radiant/src/gui_runtime/native_vello.rs`, I tightened action
  invalidation routing so high-frequency browser/search/prompt actions now use
  model+overlay invalidation without forcing static-scene dirty upfront.
- I added frame-state/test coverage in radiant for the new
  model-overlay-dirty semantics.
- In `docs/plans/active/runtime_performance_exec_plan.md`, I added Phase 7
  (invalidation scope precision + waveform adjacent latency follow-ups).
- In `docs/plans/active/todo.md`, I updated the ordered queue for Phase 7
  follow-on items.

## Work Notes

- Pending commit/push: radiant Phase 7 item 1 invalidation-scope refinement +
  active performance plan updates.
