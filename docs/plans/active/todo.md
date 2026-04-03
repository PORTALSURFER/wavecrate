# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T01:20:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 2 is active and the ranked items are being implemented sequentially.
- Items 1-5 are complete in commits `9fe71ec9`, `58e5fe24`, `2bb31ea2`, `8cf293b0`, and `7a91afd2`.
- The latest `target/perf/bench.json` run after item 5 shows `browser_focus_commit_latency.p95_us = 91`, `browser_focus_preview_latency.p95_us = 85`, `browser_filter_churn_latency.p95_us = 2477`, `hover_latency.p95_us = 2399`, `wheel_latency.p95_us = 2566`, and `waveform_pan_zoom_adjacent_latency.p95_us = 111`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again after restoring the missing `snap_override` benchmark action field.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Continue with item 6 from `tmp/perf_plan.md`: increase waveform adjacent-view cache locality instead of recomputing dense columns on pan/zoom churn.
2. Record each completed item back into `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO with validation status.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
