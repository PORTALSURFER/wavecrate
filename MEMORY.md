# Agent Memory

Last Updated: 2026-02-23T08:11:47Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-23 UTC)

- I am fixing a Windows startup regression where the native window can remain
  hidden after first present if redraw delivery stalls while hidden.
- I added a bounded startup reveal fallback in
  `vendor/radiant/src/gui_runtime/native_vello.rs`:
  after first present, if deferred-refresh reveal has not occurred by a short
  deadline, runtime force-reveals the window and requests redraw so startup
  cannot stay invisible indefinitely.
- I added regression coverage for this fallback in radiant runtime tests and
  validated with `bash scripts/ci_local.sh` (green).

- I am finishing perf-guard frame-quality threshold lock automation so repeated
  calibration runs can emit reusable env defaults directly from measured
  benchmark medians.
- I am wiring the new lock flow into `scripts/run_perf_guard.sh`, documenting
  envs/workflow in `docs/ENV_VARS.md` and `docs/performance_qa.md`, then
  running `bash scripts/ci_local.sh` before commit/push.
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
- I retried startup calibration with required valid runs:
  - `SEMPAL_PERF_GUARD_STARTUP_PROFILE=1`
  - `SEMPAL_PERF_GUARD_RUNS=5`
  - `SEMPAL_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS=1`
  - result stayed blocked in this environment:
    `startup_profile_missing_reasons: no_wayland_compositor=5`.
- I implemented startup-threshold lock automation for compositor-backed runs:
  - added `scripts/perf_startup_lock_thresholds.py` to convert
    `bench.startup_summary.json` recommendations into reusable env assignments,
  - `scripts/run_perf_guard.sh` now supports optional
    `SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT` output (with
    `SEMPAL_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS` guard) to write that env
    file automatically after successful startup-summary parsing,
  - updated `docs/ENV_VARS.md` and `docs/performance_qa.md` with the new flow.
- I re-ran item #2 A/B with larger run windows (`SEMPAL_PERF_GUARD_RUNS=5`,
  `SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS=24`) for
  `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW=1` vs `0`:
  - immediate preview ON improved `waveform_pan_zoom_adjacent_latency` median
    p95 in this batch,
  - immediate preview ON regressed `waveform_interaction_latency` median p95
    and `volume_drag_latency` median p95 in this batch.
- Decision remains unchanged: keep current immediate overlay preview behavior,
  but do not extend immediate mode to additional waveform actions until
  compositor-backed A/B runs produce stable directional evidence.
- I am closing the next perf milestone by wiring frame-quality telemetry from
  radiant redraw execution into bridge frame results:
  - `FrameBuildResult` now carries `frame_total_us`, `present_us`,
    `frame_budget_us`, `jank`, `presented`, and `missed_present`.
  - native-vello now emits those fields on every redraw attempt (including
    expected-present failures) so bridge-side metrics can classify jank and
    missed presents deterministically.
- I am keeping perf guard actionable by adding frame-quality proxy guardrails:
  - GUI latency summaries now include `frame_jank_*` and
    `missed_present_proxy_*` fields using a `16667us` frame budget baseline.
  - `scripts/run_perf_guard.sh` now prints per-scenario frame-quality proxy
    stats and supports warn/fail env thresholds for jank/missed-present ratios.
- I re-ran `bash scripts/ci_local.sh`; all checks passed with the new
  frame-quality telemetry and guardrails active.

## Work Notes

- Current focus remains runtime perf/responsiveness milestones from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Next steps are tracked in `docs/plans/active/todo.md`.
