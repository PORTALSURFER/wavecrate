# Runtime Performance Redesign Execution Plan

## Summary

- Goal: significantly improve interactive responsiveness and frame stability in
  native runtime paths, using Xilem-inspired retained/incremental update
  patterns.
- Success criteria:
  - `hover_latency` p95 < 16 ms on local perf guard runs.
  - `wheel_latency` p95 < 16 ms on local perf guard runs.
  - `waveform_interaction_latency` p95 < 10 ms on local perf guard runs.
  - Volume, waveform, map, and browser interactions remain visually smooth
    under stress loads.
- Owner: agent-driven implementation on `next`.
- Status: active, multi-day main execution plan.

## Scope

- In scope:
  - Native Vello invalidation/rebuild scheduling and overlay reuse.
  - App-core projection/model pull/cache behavior.
  - Browser/search/filter/sort pipeline recompute reduction.
  - Waveform render/caching strategy for pan/zoom/selection interactions.
  - Allocation and lock-contention reductions on hot paths.
  - Perf instrumentation and guardrail coverage improvements.
- Out of scope:
  - Feature-level UX redesign.
  - Non-performance product changes unrelated to responsiveness.

## Constraints and assumptions

- Keep behavior deterministic and existing feature semantics intact.
- Preserve docs/tests/lint guardrails and pass `scripts/ci_local.sh` before push.
- Prefer incremental, reviewable commits and measurable wins per phase.
- Use existing perf guard + benchmark harnesses as objective feedback.

## Phase Plan

### Phase 1: Fast Wins (highest ROI, lowest risk)

- [x] Introduce finer invalidation routing in native runtime so non-geometry
      changes avoid broad layout dirties.
- [x] Add overlay fingerprint checks to skip unchanged overlay rebuild/encode.
- [x] Replace expensive projection cache equality keys with revision/hash-based
      keys in app-core bridge paths.
- [x] Avoid projection-cache invalidation for motion-only action classes.
- [x] Coalesce/debounce waveform image refresh triggers for drag-heavy flows.
- [x] Reuse map DB connection/prepared statements for interactive map queries.

### Phase 2: Pipeline Recompute Reduction

- [x] Refactor browser filtering/search/sort into retained staged nodes
      (filter -> score -> sort -> viewport window).
- [x] Memoize intermediate browser results by query/filter/sort revisions.
- [x] Reuse triage/fuzzy-score caches across compatible UI updates.
- [x] Improve waveform caching to support partial/delta recompute for
      pan/zoom-adjacent views.
- [x] Expand perf benchmarks to include browser pipeline and waveform-cache
      scenarios.

### Phase 3: Deep Architecture and Throughput

- [x] Align perf-guard hover benchmark semantics with true preview-hover focus
      interactions.
- [x] Debounce deferred commit-side metadata/highlight flushes so rapid focus
      interactions avoid synchronous spike work.
- [x] Coalesce high-frequency waveform seek/cursor/selection/zoom actions in
      the native bridge before projection pulls.
- [x] Introduce explicit derived-state dependency graph with dirty propagation
      for high-churn UI subsystems.
- [x] Reduce per-frame allocation churn in shell/text paths via reusable
      buffers, fewer string clones, and stronger cache keys.
- [x] Reduce input-to-apply latency by tightening pending-input scheduling in
      native event-loop handling.
- [x] Reduce worker-pool lock contention in analysis job claim/decode queues.
- [x] Add per-stage telemetry for invalidation/rebuild rates and queue delays.

### Phase 4: Stability and Guardrail Tightening

- [x] Expand benchmark latency summaries with p99/stddev/outlier metrics for
      drift analysis.
- [x] Add perf-guard multi-run aggregation support with p95 spread reporting.
- [x] Add warning contributor ranking in perf guard output for faster
      attribution during drift events.
- [x] Promote one low-noise scenario (`map_pan_proxy_latency`) to a hard-fail
      p95 threshold while keeping high-variance scenarios warn-only.
- [x] Add stage-specific attribution fields in benchmark output for
      interaction-step internals (input/apply/pull/projection segmentation).
- [x] Promote additional scenarios to hard-fail thresholds after variance
      stabilization evidence across repeated runs.
  - [x] Promote `browser_focus_commit_latency` to a default hard-fail threshold.
  - [x] Promote `wheel_latency` to a conservative default hard-fail threshold
        and add stability-readiness tooling for future tightening.

### Phase 5: Projection + Dirty-Path Stabilization

- [x] Harden retained browser projection cache lifecycle behavior so row caches
      and selected-path lookup caches reset/rebuild deterministically.
- [x] Add targeted browser projection cache tests for:
  - visible-row revision rollover invalidation,
  - same-length selected-path updates,
  - stale cached-row column/tag refresh.
- [x] Tighten waveform dirty-path semantics with explicit overlay-vs-view
      behavior checks and helper tests.
- [x] Add derived-graph test coverage for overlay dirty-reason propagation.
- [x] Keep bridge profiling attribution counters for projection cache hit/miss
      and waveform image refresh apply/skip, and retain existing feature-gated
      metric tests.

### Phase 6: Segment-Aware Projection Refresh

- [x] Keep global projection-key correctness while switching miss handling to
      retained-model segment refresh (`status`, `browser frame`, `browser rows`,
      `map`, `waveform`) in the native bridge.
- [x] Split native browser projection into frame metadata and row-window helpers
      so bridge refresh paths can update rows independently.
- [x] Replace full cache invalidation in derived/wheel miss paths with key-only
      invalidation so retained segment state survives to the next pull.
- [x] Add segment-level bridge profiling counters/log output for hit/miss
      attribution across the retained projection segments.
- [x] Add benchmark JSON + perf-guard segment attribution output that reports
      segment-level latency/counter summaries alongside stage attribution.

### Phase 7: Invalidation Scope Precision (Next ROI)

- [x] Tighten `vendor/radiant` action invalidation routing so high-frequency
      browser/search/prompt actions use model+overlay refresh without forcing
      static-scene dirties upfront.
- [x] Validate reduced static rebuild churn via perf guard evidence and
      perf-guard reruns focused on `waveform_pan_zoom_adjacent_latency`.
- [x] Improve sempal waveform adjacent pan/zoom cache-hit rate by quantizing
      view metadata and stabilizing texture-width bucket churn.
- [x] Add a partial/delta waveform translation path (shift + edge re-render)
      to avoid full image rebuilds on pure adjacent pans.
- [x] Keep static segment rebuilds fully dirty-mask-driven across radiant
      frame segments and expand targeted tests for action-to-segment behavior.

### Phase 8: Input/Paint + Projection Hot Path Polish

- [x] Tighten volume slider drag responsiveness so drag deltas emit
      immediately and repaint does not depend on release or idle flush.
- [x] Add radiant runtime telemetry counters for rebuild causes
      (explicit static invalidations, dirty-mask static rebuilds, and bridge
      model/motion pull rebuild paths).
- [x] Reduce browser projection hot-path allocation churn by hashing retained
      selected-path lookup membership and removing `PathBuf` cloning from
      cached-row projection hits.

## Validation

- Required checks:
  - `bash scripts/ci_local.sh`
  - `bash scripts/run_perf_guard.sh`
- Bench tracking:
  - Compare p50/p95/max latency deltas before/after each milestone.
  - Track regressions with perf JSON output under `target/perf/`.

### Latest Perf-Guard Delta (2026-02-20)

- Before Phase 3.3 changes:
  - `hover_latency` p95: `53123us`
  - `browser_focus_commit_latency` p95: `17132us`
- After Phase 3.3 changes:
  - `hover_latency` p95: `3546us`
  - `browser_focus_commit_latency` p95: `4597us`

### Latest Perf-Guard Snapshot (2026-02-21)

- `hover_latency` p95: `874us`
- `wheel_latency` p95: `1120us`
- `waveform_pan_zoom_adjacent_latency` p95: `121us`
- `volume_drag_latency` p95: `140us`
- All perf-guard scenarios remained below warning thresholds.

## Milestone Exit Criteria

- Each completed milestone must:
  - keep CI green,
  - include focused tests/bench coverage for changed behavior,
  - document new guardrails/metrics when relevant,
  - and provide before/after latency evidence in commit notes or linked docs.
