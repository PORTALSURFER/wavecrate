# Clear Bug Audit Plan

Date: 2026-04-18
Status: Phase 1 complete; issues filed in Linear

Linear project:

- Sempal (`PORTALSURFER`) — https://linear.app/boostnlvp/project/sempal-7230ebfad82d

## Ordered Findings

### [x] 1. Browser feature cache serves stale row metadata after same-length reorders
- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: M
- Linear: `OPT-39` — https://linear.app/boostnlvp/issue/OPT-39/sempal-browser-feature-cache-can-show-stale-row-metadata-after-same
- Why it matters:
  Long-sample markers and other feature-derived row metadata can be projected for the wrong sample while the async refresh is still in flight.
- Evidence:
  - `src/app/controller/library/wavs/feature_cache.rs:15-24`
  - `src/app/controller/library/wavs/feature_cache.rs:153-165`
  - `src/app_core/native_shell/browser_projection/cache.rs:165-181`
- Recommended change:
  Guard feature-cache reads with the current ordered-path key, not just row count, and add a regression test for same-length reorders.

### [x] 2. Browser row labels can remain stale across same-length loads/reorders
- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: M
- Linear: `OPT-40` — https://linear.app/boostnlvp/issue/OPT-40/sempal-browser-row-labels-can-stay-stale-when-a-load-or-reorder-keeps
- Why it matters:
  The browser can show the wrong sample names, and the retained projection cache can preserve those wrong labels past the load that introduced them.
- Evidence:
  - `src/app/controller/library/wavs/browser_search/cache.rs:169-206`
  - `src/app/controller/ui/loading.rs:161`
  - `src/app/controller/ui/loading.rs:179-183`
  - `src/app_core/native_shell/browser_projection/cache.rs:188-193`
- Recommended change:
  Invalidate labels before projection work on load, and key label reuse to row identity instead of only entry count.

### [x] 3. Sync browser search score cache reuses stale fuzzy scores after same-length reorders
- Classification: Bug fix
- Confidence: Medium-High
- ROI: Medium
- Effort: M
- Linear: `OPT-41` — https://linear.app/boostnlvp/issue/OPT-41/sempal-synchronous-browser-search-score-cache-reuses-stale-fuzzy
- Why it matters:
  Search ordering and matched-row subsets can reflect the previous path layout until some unrelated invalidation happens.
- Evidence:
  - `src/app/controller/library/wavs/browser_search/cache.rs:65-83`
  - `src/app/controller/library/wavs/search_scoring.rs:25-61`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages/source_cache.rs:129-161`
- Recommended change:
  Mirror the worker path by tracking an ordered-path fingerprint and clearing sync query-score reuse when that fingerprint changes.

## Open Questions / Missing Definitions

- None for filing the bugs. The conservative fix for each item is to hide or invalidate stale index-aligned cache entries until the matching ordered snapshot is available.

## Rejected Ideas

- `[-]` File generic cache-cleanup issues without a repro shape.
  - Why it was considered: multiple retained caches in this area share similar patterns.
  - Why it was rejected: the three issues above already have direct repository evidence and specific user-visible failure modes; broader cleanup tickets would be too vague.
