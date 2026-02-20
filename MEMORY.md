# Agent Memory

Last Updated: 2026-02-20T18:48:39Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the volume-drag responsiveness/perf milestone.
- In `vendor/radiant/src/gui_runtime/native_vello.rs`, I now coalesce
  `SetVolume` drag actions to one queued update per frame and flush the latest
  value before `CommitVolumeSetting` on drag release.
- In `src/bin/bench/gui.rs` and `src/bin/bench/gui/interactions.rs`, I added
  `volume_drag_latency` benchmark coverage and staged attribution wiring.
- In `scripts/run_perf_guard.sh` and docs, I added `volume_drag_latency`
  scenario thresholds and documentation.
- Full `bash scripts/ci_local.sh` is green for this change set.

## Work Notes

- Pending commit/push: volume-drag coalescing + benchmark/perf-guard updates.
