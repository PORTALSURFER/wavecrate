# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T13:40:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is in progress. Item 1 is complete in commit `3c21e5ac`; items 2-6 remain pending.
- The latest item-1 validation perf run is `scripts/run_perf_guard.ps1`: `browser_filter_churn_latency.p95_us = 2617`, `hover_latency.p95_us = 4288`, `wheel_latency.p95_us = 3169`, `browser_focus_preview_latency.p95_us = 179`, and `browser_focus_commit_latency.p95_us = 228`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Implement item 2 from `tmp/perf_plan.md`: remove UI-thread wav page loads from browser row projection and BPM preload.
2. Validate item 2 with focused tests, `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1`.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
