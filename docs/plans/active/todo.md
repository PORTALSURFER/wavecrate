# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T14:06:07+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is in progress. Items 1-5 are complete in commits `3c21e5ac`, `362dd5bc`, `4ee6ad01`, `8a9ca37e`, and `43373e1f`; item 6 remains pending.
- The latest item-5 validation lane passed focused startup-audio tests and `scripts/ci_agent.ps1`. The latest perf run remains item 3: `browser_filter_churn_latency.p95_us = 3410`, `browser_query_churn_latency.p95_us = 62`, `browser_sort_toggle_latency.p95_us = 62`, `hover_latency.p95_us = 2296`, `wheel_latency.p95_us = 2442`, `browser_focus_preview_latency.p95_us = 51`, and `browser_focus_commit_latency.p95_us = 58`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Implement item 6 from `tmp/perf_plan.md`: reduce retained renderer composition churn and transient browser row text allocations.
2. Validate item 6 with focused rendering/browser-row tests, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
