# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-23T11:40:12Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).

## Next tasks (ordered)

1. Run startup-profile calibration on a compositor-backed host and lock
   threshold env defaults from `startup_first_paint_recommended` output
   (use `SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT` for one-shot lock-file output).
2. Repeat immediate waveform-preview A/B on a compositor-backed host with
   larger run windows to reduce variance, then decide whether to extend
   immediate apply beyond overlay actions.
3. Maintain handoff hygiene on every milestone commit:
   update `AGENTS.md`, `MEMORY.md`, and this queue in the same change set.

## Done recently

- Completed ROI item #9 waveform projection clone-elision:
  - waveform projection/model payloads now use `Arc<ImageRgba>`.
  - radiant `ImageRgba` pixels now use `Arc<[u8]>`.
  - native-shell projection cache hits now reuse shared payload handles.
- Completed ROI item #8 selected-path lookup cache gating by selection revision.
- Completed ROI item #5 folder-filter and triage cache reuse in browser pipeline/search worker.
