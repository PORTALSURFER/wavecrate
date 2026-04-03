# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T03:25:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 2 is complete and the ranked items are fully implemented in strict ROI order.
- Items 1-7 are complete in commits `9fe71ec9`, `58e5fe24`, `2bb31ea2`, `8cf293b0`, `7a91afd2`, `ffe0651c`, `c615c664`, and `75f8294d`.
- The latest `target/perf/bench.json` run for the completed lane shows `browser_filter_churn_latency.p95_us = 2999`, `browser_query_churn_latency.p95_us = 74`, `browser_sort_toggle_latency.p95_us = 77`, `hover_latency.p95_us = 2545`, `wheel_latency.p95_us = 2694`, `browser_focus_preview_latency.p95_us = 60`, `browser_focus_commit_latency.p95_us = 57`, and `waveform_pan_zoom_adjacent_latency.p95_us = 165`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again after restoring the missing `snap_override` benchmark action field.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Treat `tmp/perf_plan.md` as the completed performance execution record until the user opens a new lane.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Sync this TODO, `AGENTS.md`, and `MEMORY.md` only when a new active lane starts.
