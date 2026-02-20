# Agent Memory

Last Updated: 2026-02-20T17:31:06Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing the next GUI performance milestone: bridge-provided static
  segment revisions replacing per-frame segment model hashing in retained scene
  cache keys.
- In `vendor/radiant/src/app/mod.rs`, I added `SegmentRevisions` and extended
  `NativeAppBridge` with `take_segment_revisions`.
- In `src/app_core/native_bridge.rs`, I now track monotonic revision counters
  from projection dirty masks and expose them to the runtime.
- In `vendor/radiant/src/gui_runtime/native_vello.rs`, static segment scene
  fingerprints now use bridge revisions, including a one-shot conservative
  rebuild fallback when revisions are not provided.
- Full `bash scripts/ci_local.sh` is green for this change set.

## Work Notes

- Pending commit/push: bridge segment revision cache-key milestone.
