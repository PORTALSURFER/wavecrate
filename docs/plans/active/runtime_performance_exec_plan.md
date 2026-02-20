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

- [ ] Introduce explicit derived-state dependency graph with dirty propagation
      for high-churn UI subsystems.
- [ ] Reduce per-frame allocation churn in shell/text paths via reusable
      buffers, fewer string clones, and stronger cache keys.
- [x] Reduce input-to-apply latency by tightening pending-input scheduling in
      native event-loop handling.
- [ ] Reduce worker-pool lock contention in analysis job claim/decode queues.
- [ ] Add per-stage telemetry for invalidation/rebuild rates and queue delays.

## Validation

- Required checks:
  - `bash scripts/ci_local.sh`
  - `bash scripts/run_perf_guard.sh`
- Bench tracking:
  - Compare p50/p95/max latency deltas before/after each milestone.
  - Track regressions with perf JSON output under `target/perf/`.

## Milestone Exit Criteria

- Each completed milestone must:
  - keep CI green,
  - include focused tests/bench coverage for changed behavior,
  - document new guardrails/metrics when relevant,
  - and provide before/after latency evidence in commit notes or linked docs.
