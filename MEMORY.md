# Agent Memory

Last Updated: 2026-02-22T11:50:41Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-22 UTC)

- I attacked startup-baseline item #1 by tightening startup capture and
  calibration tooling:
  - `scripts/run_perf_guard.sh` now prebuilds `sempal` for startup captures,
    supports minimum-valid startup run requirements, and forwards startup spread
    thresholds to summary parsing,
  - `scripts/perf_startup_summary.py` now emits richer startup stats
    (median/p95/p99/max/spread), missing-reason classification, and calibrated
    threshold recommendations (`startup_first_paint_recommended`).
- I ran multi-run startup capture attempts and confirmed this environment cannot
  produce first-present samples because no Wayland compositor is available
  (`no_wayland_compositor`), so guard calibration now reports that reason
  explicitly instead of silently producing empty output.
- I updated startup profiling docs in `docs/ENV_VARS.md` and
  `docs/performance_qa.md` for min-valid-run enforcement and calibration flow.
- I started item #2 and implemented the first queue-splitting pass in
  `app_core::native_bridge`:
  - waveform overlay preview actions (`SetWaveformCursor`,
    `SetWaveformSelectionRange`, `ClearWaveformSelection`) now apply
    immediately for smoother drag/selection feedback,
  - heavier waveform commit actions (`SeekWaveform`, zoom actions) remain
    queued/coalesced to protect apply-stage cost,
  - added regression tests proving preview actions bypass queueing while seek
    stays queued.
- I validated with `bash scripts/ci_local.sh`; all checks passed.
- I measured A/B perf impact with
  `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW=0/1` under identical
  `run_perf_guard` settings:
  - `waveform_interaction_latency` was neutral-to-better with immediate preview
    in the second pass,
  - `waveform_pan_zoom_adjacent_latency` showed higher variance/outliers with
    immediate preview in one pass.
- Decision: keep immediate overlay preview behavior for UX, but do not extend
  immediate mode to additional waveform actions yet until we have lower-noise
  comparative runs on the compositor-backed target host.
- I implemented startup artifact suppression in radiant native-vello startup:
  - first present remains hidden while deferred startup model refresh is still
    pending,
  - the window reveal now happens only after a stable post-refresh present,
    avoiding the black/placeholder startup flash.
- I re-ran `bash scripts/ci_local.sh`; all checks passed.

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
