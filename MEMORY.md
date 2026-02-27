# Agent Memory

Last Updated: 2026-02-27T11:13:47Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am maintaining handoff clarity for stateless sessions while continuing
  runtime responsiveness/performance execution.
- The active mission remains runtime responsiveness/performance redesign from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Startup-profile calibration on a compositor-backed host is complete with 5/5
  valid runs (`target/perf/bench.startup_calibration2.startup_summary.json`),
  and startup threshold defaults are locked in
  `target/perf/startup_thresholds.lock.env`.
- Immediate waveform-preview A/B with larger windows is complete (7-run median
  comparison); results favor preview-off latencies for waveform scenarios, so
  immediate-apply scope remains limited to overlay actions for now.
- The latest shipped milestone is a first browser-projection optimization pass:
  single-selection lookup fast path + row-slot string rewrite short-circuit in
  `src/app_core/native_shell.rs` / `src/app/controller.rs` integration paths.
- Local preflight and CI are green in this session (`bash scripts/run_agent_request.sh`
  and `bash scripts/ci_local.sh`), with perf-guard warnings at warn-only levels.

## Immediate Next Actions

1. Reduce compositor-run warning drift in browser-heavy scenarios
   (`hover_latency`, `wheel_latency`, `browser_filter_churn_latency`) from the
   latest 7-run perf guard evidence.
2. Root-cause projection-stage spikes seen in waveform interaction outliers
   under immediate-preview-on runs before revisiting immediate-apply scope.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` synchronized
   on every milestone.

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
