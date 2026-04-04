# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T18:26:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is in progress. Items 1-2 are complete in commits `3c91fbef` and `dacfedac`, with vendor/radiant item 1 support in `e5c91739`.
- The current validation lane for item 2 passed focused waveform/polling tests, `scripts/ci_agent.ps1`, and `scripts/run_perf_guard.ps1`. The latest perf run reports `browser_filter_churn_latency.p95_us = 3333`, `browser_query_churn_latency.p95_us = 66`, `browser_sort_toggle_latency.p95_us = 86`, `hover_latency.p95_us = 2281`, `wheel_latency.p95_us = 2603`, `browser_focus_preview_latency.p95_us = 51`, `browser_focus_commit_latency.p95_us = 57`, and `waveform_interaction_latency.p95_us = 187`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Implement item 3 from `tmp/perf_plan.md`: remove full-source path and embedding scans from loaded-similarity workflows.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
