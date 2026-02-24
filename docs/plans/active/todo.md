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

1. Investigate compositor-run warning drift in browser/scroll interactions
   (`hover_latency`, `wheel_latency`, `browser_filter_churn_latency`) and
   prioritize projection-stage reductions from latest 7-run evidence.
2. Keep immediate waveform preview scope limited to overlay actions; revisit
   any wider immediate-apply scope only after a dedicated UX+perf gate.
3. Maintain handoff hygiene on every milestone commit:
   update `AGENTS.md`, `MEMORY.md`, and this queue in the same change set.

## Done recently

- Implemented first browser-projection drift reduction pass for item 1:
  - added single-selection fast path for retained selected-path lookup
    (`ProjectedSelectedPathsLookup::Single`) to avoid dense bitset rebuild
    churn on focus/wheel-heavy interactions.
  - reduced browser row rewrite churn by skipping label/bucket string rewrites
    when row slot content is unchanged.
  - validated with `bash scripts/run_perf_guard.sh` artifact
    `target/perf/bench.post_item1.clean2.json` (clean run showed lower
    p95s for filter/wheel/hover versus pre-change local sample).
  - CI remained green (`bash scripts/ci_local.sh`).
- Completed startup-profile threshold calibration on compositor-backed runs:
  - `target/perf/bench.startup_calibration2.startup_summary.json`
  - 5/5 valid startup profiles
  - locked thresholds in `target/perf/startup_thresholds.lock.env`:
    - `SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_MS=4515`
    - `SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_MS=7224`
    - `SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS=2187`
    - `SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS=3937`
- Completed immediate waveform-preview A/B with larger run windows
  (`SEMPAL_PERF_GUARD_RUNS=7`) on compositor-backed runs:
  - on (`target/perf/wave_preview_on_calib.json`) vs off
    (`target/perf/wave_preview_off_calib.json`) showed lower waveform p95/p99
    when off; decision recorded: do not extend immediate-apply scope beyond
    overlay actions at this time.
- Completed ROI item #9 waveform projection clone-elision:
  - waveform projection/model payloads now use `Arc<ImageRgba>`.
  - radiant `ImageRgba` pixels now use `Arc<[u8]>`.
  - native-shell projection cache hits now reuse shared payload handles.
- Completed ROI item #8 selected-path lookup cache gating by selection revision.
- Completed ROI item #5 folder-filter and triage cache reuse in browser pipeline/search worker.
