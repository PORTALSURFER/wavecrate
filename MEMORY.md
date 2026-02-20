# Agent Memory

Last Updated: 2026-02-20T15:21:51Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing performance plan item #2: retained static-scene segment
  composition for the native `winit + vello` runtime.
- In `vendor/radiant/src/gui/native_shell/state.rs`, I added static segment
  definitions (`status`, `browser frame`, `browser rows`, `map`, `waveform`,
  `global`) plus segmented static-frame partition builders and coverage tests.
- In `vendor/radiant/src/gui_runtime/native_vello.rs`, I added per-segment
  scene caches keyed by `(segment, viewport bits, style signature, segment model
  signature)` and wired incremental static-scene composition under
  `SEMPAL_NATIVE_INCREMENTAL_FRAME_PIPELINE`.
- Full `bash scripts/ci_local.sh` is green for this change set.

## Work Notes

- Pending commit/push: plan #2 retained static segment composition milestone.
