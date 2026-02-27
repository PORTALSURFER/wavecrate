# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-24T14:11:37Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).

## Next tasks (ordered)

1. Recalibrate the tracked startup threshold lock file on a compositor-backed
   host using `bash scripts/calibrate_startup_thresholds.sh`, then verify
   `scripts/perf_locks/startup_thresholds.env` values against
   `startup_first_paint_recommended`.
2. Repeat immediate waveform-preview A/B on a compositor-backed host with
   larger run windows to reduce variance, then decide whether to extend
   immediate apply beyond overlay actions.
3. Maintain handoff hygiene on every milestone commit:
   update `AGENTS.md`, `MEMORY.md`, and this queue in the same change set.

## Done recently

- Added tracked startup threshold lock-file defaults at
  `scripts/perf_locks/startup_thresholds.env` and auto-loading in
  `scripts/run_perf_guard.sh`.
- Completed ROI item #9 waveform projection clone-elision:
  - waveform projection/model payloads now use `Arc<ImageRgba>`.
  - radiant `ImageRgba` pixels now use `Arc<[u8]>`.
  - native-shell projection cache hits now reuse shared payload handles.
- Completed ROI item #8 selected-path lookup cache gating by selection revision.
- Completed ROI item #5 folder-filter and triage cache reuse in browser pipeline/search worker.
