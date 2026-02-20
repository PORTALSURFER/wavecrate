# Agent Memory

Last Updated: 2026-02-20T14:23:07Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am implementing and shipping the Phase 7 browser churn tail-cut +
  attribution hardening milestone for runtime performance.
- In `src/app/controller/library/wavs/browser_search.rs`, I made browser-search
  async offload threshold configurable (`SEMPAL_BROWSER_SEARCH_OFFLOAD_THRESHOLD`)
  and reduced query-score recompute allocations by scoring from cached labels.
- In `src/app/controller/library/wavs/browser_pipeline.rs`, I removed hot-path
  row-vector clones in filter/score stages by iterating cached row arrays in place.
- In `src/bin/bench/gui/interactions.rs` and `src/bin/bench/gui.rs`, I converted
  browser filter/query/sort churn scenarios to staged benchmarks and now emit
  stage attribution for these three scenarios.
- In `scripts/run_perf_guard.sh`, I added the three churn scenarios to drift
  reporting and made scenario parsing tolerant when a report omits keys.
- In `docs/ENV_VARS.md`, I documented the new browser-search threshold and perf
  guard churn threshold overrides.
- Full `bash scripts/ci_local.sh` is green after these changes.

## Work Notes

- Pending commit/push: Phase 7 browser churn tail-cut and attribution hardening.
