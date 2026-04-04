# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T14:38:17+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is complete. Items 1-6 are complete in commits `3c21e5ac`, `362dd5bc`, `4ee6ad01`, `8a9ca37e`, `43373e1f`, and vendor/radiant `9e2bc927`.
- The completion validation lane passed focused `vendor/radiant` tests, `scripts/ci_quick.ps1`, `scripts/run_perf_guard.ps1`, and `scripts/ci_agent.ps1`. The latest perf run reports `browser_filter_churn_latency.p95_us = 2398`, `browser_query_churn_latency.p95_us = 63`, `browser_sort_toggle_latency.p95_us = 68`, `hover_latency.p95_us = 2751`, `wheel_latency.p95_us = 2273`, `browser_focus_preview_latency.p95_us = 58`, and `browser_focus_commit_latency.p95_us = 64`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Treat `tmp/perf_plan.md` as the completed runtime-performance execution record until the user opens a new performance lane.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
