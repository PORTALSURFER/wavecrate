# Agent Memory

Last Updated: 2026-02-20T14:56:35Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the next GUI performance milestone: bridge-driven dirty
  segment tracking for incremental native frame updates.
- I added `DirtySegments` to the Radiant bridge contract and wired
  `SempalNativeBridge` projection caching to return segment deltas per pull.
- I updated native `winit + vello` runtime rebuild gating to support
  `SEMPAL_NATIVE_INCREMENTAL_FRAME_PIPELINE`, so static rebuilds can be skipped
  when the bridge reports no static segment changes.
- I documented `SEMPAL_NATIVE_INCREMENTAL_FRAME_PIPELINE` in
  `docs/ENV_VARS.md`.
- Full `bash scripts/ci_local.sh` is green for this change set.

## Work Notes

- Pending commit/push: incremental dirty-segment frame pipeline milestone.
