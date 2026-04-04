# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T19:18:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is in progress. Items 1-4 are complete in commits `3c91fbef`, `dacfedac`, `d573ddeb`, and `849f0cf6`, with vendor/radiant item 1 support in `e5c91739`.
- The current validation lane for item 4 passed focused browser-selection tests, `scripts/ci_agent.ps1`, and `scripts/run_perf_guard.ps1`. The latest perf run reports `browser_filter_churn_latency.p95_us = 2336`, `browser_query_churn_latency.p95_us = 157`, `browser_sort_toggle_latency.p95_us = 160`, `hover_latency.p95_us = 2274`, `wheel_latency.p95_us = 2554`, `browser_focus_preview_latency.p95_us = 180`, `browser_focus_commit_latency.p95_us = 179`, and `waveform_interaction_latency.p95_us = 205`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Implement item 5 from `tmp/perf_plan.md`: deduplicate optimistic metadata mutation batches and loaded-audio membership checks.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
