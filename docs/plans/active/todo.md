# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T23:02:52+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 2 is active and the ranked items are being implemented sequentially.
- Items 1-2 are complete in `vendor/radiant` commits `9fe71ec9` and `58e5fe24`.
- The latest `target/perf/bench.json` run after item 2 shows `hover_latency.p95_us = 3042`, `wheel_latency.p95_us = 2622`, `browser_focus_commit_latency.p95_us = 96`, and `waveform_pan_zoom_adjacent_latency.p95_us = 89`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is green again after restoring the missing `snap_override` benchmark action field.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Continue with item 3 from `tmp/perf_plan.md`: stop cloning the entire retained native app model on every bridge projection miss.
2. Record each completed item back into `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO with validation status.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
