# Agent Memory

Last Updated: 2026-02-24T11:33:24Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am executing the runtime responsiveness/performance redesign and closing
  the latest calibration/decision tasks from the active queue.
- The active mission remains runtime responsiveness/performance redesign from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Startup-profile calibration on a compositor-backed host is complete with 5/5
  valid runs (`target/perf/bench.startup_calibration2.startup_summary.json`),
  and startup threshold defaults are locked in
  `target/perf/startup_thresholds.lock.env`.
- Immediate waveform-preview A/B with larger windows is complete (7-run median
  comparison); results favor preview-off latencies for waveform scenarios, so
  immediate-apply scope remains limited to overlay actions for now.

## Immediate Next Actions

1. Reduce compositor-run warning drift in browser-heavy scenarios
   (`hover_latency`, `wheel_latency`, `browser_filter_churn_latency`) from the
   latest 7-run perf guard evidence.
2. Root-cause projection-stage spikes seen in waveform interaction outliers
   under immediate-preview-on runs before revisiting immediate-apply scope.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md`
   synchronized on every milestone commit.

## Work Notes

- Detailed execution and rationale live in:
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Short ordered queue lives in:
  `docs/plans/active/todo.md`.
- Latest calibration artifacts:
  - `target/perf/bench.startup_calibration2.startup_summary.json`
  - `target/perf/startup_thresholds.lock.env`
  - `target/perf/wave_preview_on_calib.json`
  - `target/perf/wave_preview_off_calib.json`
