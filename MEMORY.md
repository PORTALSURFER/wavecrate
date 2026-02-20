# Agent Memory

Last Updated: 2026-02-20T19:11:30Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the waveform apply-path fast-lane performance milestone.
- In `src/app/controller/playback/mod.rs`, I added no-op fast paths for
  unchanged cursor/selection/zoom updates while waveform focus is already
  active.
- In `src/app_core/native_bridge.rs`, queued waveform flush now dedupes
  redundant cursor+seek pairs and wraps the flush in one outer waveform refresh
  batch scope.
- In `scripts/run_perf_guard.sh`, I added
  `waveform_pan_zoom_adjacent_latency` as a warn-threshold scenario.
- In docs (`docs/ENV_VARS.md`, `docs/performance_qa.md`), I documented the new
  adjacent waveform perf guard knobs and QA coverage.
- Full `bash scripts/ci_local.sh` is green for this change set.

## Work Notes

- Pending commit/push: waveform apply-path fast-lane + adjacent waveform
  perf-guard milestone.
