# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T17:13:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is complete. Items 1-6 are complete in commits `fc2fca4e`, `ef649778`, vendor/radiant `d13e5f55`, `ca24b6d3`, `18f8d5d5`, `9009d402`, and vendor/radiant `427e115b`, with the superproject vendor bump in `53ea4684`.
- The latest `target/perf/bench.json` run after item 6 shows `browser_filter_churn_latency.p95_us = 2767`, `hover_latency.p95_us = 2896`, `wheel_latency.p95_us = 2830`, `browser_focus_preview_latency.p95_us = 153`, and `browser_focus_commit_latency.p95_us = 143`.
- The post-item-6 perf guard completed without warnings and kept the browser interaction tail materially below the Phase 1 baseline recorded in `tmp/perf_plan.md`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is active.

## Next tasks (ordered)

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record until the user opens a new performance lane.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Start a new ordered queue only if the user explicitly reopens performance work or selects another lane.
