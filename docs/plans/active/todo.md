# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-05T00:05:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is complete. Items 1-8 are complete in commits `3c91fbef`, `dacfedac`, `d573ddeb`, `849f0cf6`, `faf927d8`, `4ac3945e`, `0efad4c2`, and `46a7168b`, with vendor/radiant support in `e5c91739` and `2f53bf98`.
- The final validation lane passed focused item-7 tests, `vendor/radiant` library compile, `scripts/ci_agent.ps1`, and `scripts/run_perf_guard.ps1`. The latest perf run reports `browser_filter_churn_latency.p95_us = 2075`, `browser_query_churn_latency.p95_us = 176`, `browser_sort_toggle_latency.p95_us = 157`, `hover_latency.p95_us = 2809`, `wheel_latency.p95_us = 2511`, `browser_focus_preview_latency.p95_us = 142`, `browser_focus_commit_latency.p95_us = 151`, and `waveform_interaction_latency.p95_us = 1444`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record unless the user opens a new performance lane.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
