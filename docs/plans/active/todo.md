# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-04T01:10:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 2 is active and the ranked items are being implemented sequentially.
- The audit baseline is `target/perf/bench.json`, where the biggest current p95 costs are hover/wheel projection work, browser focus commit, adjacent waveform pan/zoom, and the filter-lane tail.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again after restoring the missing `snap_override` benchmark action field.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Continue implementing `tmp/perf_plan.md` strictly in ranked order.
2. Record each completed item back into `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO with validation status.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
