# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-22T12:05:32Z
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
2. Repeat waveform immediate-preview A/B on compositor-backed host with larger
   run windows to reduce variance, then decide whether to extend immediate path
   to additional waveform actions.
3. Keep `MEMORY.md` and this queue updated in every perf milestone commit.

## Done recently

- Completed ROI batch items 1-3:
  - immediate volume drag emits in radiant for smooth in-drag rendering,
  - rebuild-cause profiler counters/reporting in radiant
    (`explicit_static`, `dirty_mask_static`, bridge pull rebuild counts),
  - browser projection low-allocation pass in sempal (selected-path hash lookup
    cache and reduced cached-row cloning on projection hits).
- Completed next ROI follow-up batch:
  - GUI bench JSON now includes `interaction_rebuild_cause_attribution`.
  - Perf guard now prints rebuild-cause counters per scenario.
  - Native bridge now reuses retained browser row-model `Vec` allocation.
  - Radiant cursor-move events process immediately when layout is ready.
- Completed next plugin/runtime milestone:
  - GUI bench now probes real retained segment hit/miss counters via
    `native_bridge` projection-cache measurement loops.
  - Perf guard now prints non-zero segment hit/miss values from those probes.
  - Radiant waveform drag now emits immediate updates while dragging.
  - Radiant map focus drag now emits immediate focus updates while dragging.
- Completed browser projection allocation-churn milestone:
  - retained browser-row cache now stores typed cache entries with precomputed
    selected-path lookup hashes,
  - browser row projection now reuses row-model slots and mutates row/bucket
    strings in place instead of rebuilding row objects every frame.
- Completed immediate-input + startup lean-path milestone:
  - wheel browser-focus actions now emit immediately in native runtime instead
    of waiting for pending flush boundaries,
  - startup first frame now defers full model/overlay pulls until after first
    successful present.
- Completed startup timing + remaining runtime queue cleanup milestone:
  - added startup first-paint timing summary hooks (including deferred full
    model refresh) for log/benchmark tracking,
  - removed runtime wheel pending-flush queue path so wheel actions emit
    immediately,
  - process queued cursor movement immediately when layout becomes available.
- Completed startup guard ingestion + bridge immediate-focus milestone:
  - perf guard now supports optional startup profiling capture and startup
    summary output (`SEMPAL_PERF_GUARD_STARTUP_PROFILE` and summary sidecar),
  - added `scripts/perf_startup_summary.py` to parse startup profile logs and
    enforce warning/fail thresholds for first-present latency,
  - native bridge now applies `MoveBrowserFocus` actions immediately instead of
    queueing them for later flush.
- Completed startup calibration hardening milestone:
  - startup summary now reports first-present median/p95/p99/max/spread and
    emits calibrated threshold recommendations,
  - startup summary now classifies missing-capture reasons (for example
    `no_wayland_compositor`) and supports minimum-valid-run enforcement,
  - perf guard now prebuilds startup binary for capture runs and supports
    startup spread thresholds + required valid-run gate.
- Completed waveform queue split milestone (first pass):
  - waveform overlay preview actions now apply immediately in the native bridge
    (`SetWaveformCursor`, selection-range set/clear),
  - seek/zoom actions remain queued/coalesced to cap apply-stage cost,
  - added focused bridge tests for immediate preview behavior and queued seek
    behavior.
- Completed waveform immediate-preview A/B evaluation milestone (local host):
  - measured `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW=0/1` under
    identical perf-guard settings,
  - observed neutral-to-better direct `waveform_interaction_latency` with
    immediate preview in one pass,
  - observed higher variance on adjacent waveform pan/zoom in one pass,
  - decision: keep immediate overlay preview behavior, but do not extend
    immediate mode beyond overlay actions until compositor-backed A/B repeats.
- Completed startup visual-cleanup milestone:
  - native-vello startup now keeps the window hidden through the deferred
    startup model refresh,
  - window reveal now occurs only after the first stable post-refresh present
    to avoid black/placeholder startup artifacts.
- Completed startup-threshold lock-automation milestone:
  - added `scripts/perf_startup_lock_thresholds.py` to write startup threshold
    env assignments from startup summary recommendations,
  - `scripts/run_perf_guard.sh` now supports
    `SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT` plus
    `SEMPAL_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS`,
  - local startup calibration rerun remained blocked in this environment with
    `no_wayland_compositor`, so compositor-backed lock run is still pending.
- Completed waveform immediate-preview larger-window A/B rerun milestone
  (local host):
  - ran `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW=1/0` with
    `SEMPAL_PERF_GUARD_RUNS=5` and standard interaction windows,
  - observed mixed directional deltas (ON improved adjacent pan/zoom p95 but
    regressed direct waveform interaction p95 in this batch),
  - decision unchanged: keep current split, do not extend immediate mode beyond
    overlay actions until compositor-backed repeats are stable.
- Completed ROI item #1: switched waveform multi-step zoom to single-pass math with regression coverage.
- Completed Phase 7 item 1 foundation: tightened radiant invalidation scope routing so
  high-frequency browser/search/prompt actions use model+overlay invalidation.
- Completed Phase 7 static rebuild scope follow-up: model-refresh static
  rebuilds now stay dirty-mask-driven unless explicit static invalidation is
  requested, with focused radiant tests.
- Completed browser projection micro-optimization: removed visible-row window
  copy in `project_browser_rows_model` and switched to direct visible index
  reads with bounded pre-allocation.
- Completed Phase 7 waveform cache-hit improvement: quantized waveform view
  metadata matching and stabilized texture-width bucketing.
- Completed Phase 7 delta pan milestone: added waveform translation reuse
  (shift + edge patch render) with adjacent-pan regression coverage.
- Completed deferred waveform seek-commit milestone: native seek interactions
  now queue/debounce replay seeks, reducing waveform interaction apply-stage
  cost with regression coverage.
