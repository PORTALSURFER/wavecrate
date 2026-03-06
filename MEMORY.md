# Agent Memory

Last Updated: 2026-03-05T19:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have added a persistent waveform decode cache under the app root so decoded waveform payloads and transient markers can be reused across app restarts.
- The controller now falls back from the in-memory audio cache to the persistent waveform cache before decoding a waveform again.
- I refreshed waveform cache invalidation so edited/reloaded samples clear both in-memory and persistent entries.
- Focused cache-path checks are limited by unrelated existing compile failures in `src/updater/mod.rs` and `src/bin/sempal-installer/cleanup.rs`.
- `tmp/cleanup_plan.md` remains the pending cleanup backlog; cleanup Phase 2 is still waiting for explicit user confirmation.

## Immediate Next Actions

1. Resolve the unrelated compile blockers if a green full CI run is required before commit/push.
2. Re-run full local CI after those unrelated blockers are fixed.
3. Return to the pending cleanup confirmation flow after the waveform caching request is complete.

## Work Notes

- Waveform caching work touches controller playback/cache loading paths and `src/app/controller/playback/persistent_waveform_cache.rs`.
- Active cleanup backlog (pending): `tmp/cleanup_plan.md`.
- Runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
