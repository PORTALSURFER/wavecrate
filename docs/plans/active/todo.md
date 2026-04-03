# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T23:55:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 2 is active and the ranked items are being implemented sequentially.
- Items 1-4 are complete in commits `9fe71ec9`, `58e5fe24`, `2bb31ea2`, and `8cf293b0`.
- The latest `target/perf/bench.json` run after item 4 shows `browser_filter_churn_latency.p95_us = 2687`, `browser_query_churn_latency.p95_us = 99`, `browser_sort_toggle_latency.p95_us = 98`, `hover_latency.p95_us = 2595`, `wheel_latency.p95_us = 2899`, `browser_focus_commit_latency.p95_us = 93`, and `waveform_pan_zoom_adjacent_latency.p95_us = 92`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again after restoring the missing `snap_override` benchmark action field.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Continue with item 5 from `tmp/perf_plan.md`: split browser commit focus into an immediate UI update and deferred heavy side effects.
2. Record each completed item back into `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO with validation status.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
