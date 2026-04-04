# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T09:35:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-04.
- Phase 2 is active. Items 1-2 are complete in commits `fc2fca4e` and `ef649778`, and item 3 is next.
- The latest `target/perf/bench.json` run after item 2 shows `browser_filter_churn_latency.p95_us = 2700`, `hover_latency.p95_us = 3065`, `wheel_latency.p95_us = 3195`, `waveform_interaction_latency.p95_us = 189`, and `waveform_pan_zoom_adjacent_latency.p95_us = 188`.
- The post-item-2 perf guard completed without warnings and kept the browser interaction tail materially below the Phase 1 baseline recorded in `tmp/perf_plan.md`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is active.

## Next tasks (ordered)

1. Implement item 3 from `tmp/perf_plan.md` next, then validate, update the plan, commit, and push.
2. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
3. Continue the remaining performance items strictly in ROI order after each validated commit/push pair.
