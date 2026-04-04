# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T09:12:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is active. Item 1 is complete in commit `fc2fca4e` and item 2 is next.
- The latest `target/perf/bench.json` run after item 1 shows `browser_filter_churn_latency.p95_us = 2421`, `hover_latency.p95_us = 2602`, `wheel_latency.p95_us = 3159`, `waveform_interaction_latency.p95_us = 246`, and `waveform_pan_zoom_adjacent_latency.p95_us = 147`.
- The post-item-1 perf guard completed without warnings and materially reduced the browser filter, hover, and wheel tails versus the Phase 1 baseline captured in `tmp/perf_plan.md`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is active.

## Next tasks (ordered)

1. Implement item 2 from `tmp/perf_plan.md` next, then validate, update the plan, commit, and push.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Continue the remaining performance items strictly in ROI order after each validated commit/push pair.
