# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-22T09:18:47Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).

## Next tasks (ordered)

1. Extend immediate drag-path behavior to remaining continuous controls that
   still queue updates to release/flush boundaries.
2. Keep startup first-frame path lean by deferring non-critical projection/cache
   work until after initial present.
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
