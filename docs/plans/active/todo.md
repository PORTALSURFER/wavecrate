# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T13:20:57+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is in progress. Items 1-2 are complete in commits `3c21e5ac` and `362dd5bc`; items 3-6 remain pending.
- The latest item-2 validation perf run is `scripts/run_perf_guard.ps1`: `browser_filter_churn_latency.p95_us = 2416`, `browser_query_churn_latency.p95_us = 159`, `browser_sort_toggle_latency.p95_us = 154`, `hover_latency.p95_us = 2351`, `wheel_latency.p95_us = 2508`, `browser_focus_preview_latency.p95_us = 152`, and `browser_focus_commit_latency.p95_us = 172`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Implement item 3 from `tmp/perf_plan.md`: move feature-refresh scheduling and base-stage DB revision probes out of the hot row-projection path.
2. Validate item 3 with focused tests, `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
