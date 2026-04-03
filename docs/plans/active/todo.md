# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-03T22:36:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the refreshed runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-03.
- Phase 1 is complete and Phase 2 is waiting for explicit user confirmation.
- The audit baseline is `target/perf/bench.json`, where the biggest current p95 costs are hover/wheel projection work, browser focus commit, adjacent waveform pan/zoom, and the filter-lane tail.
- `powershell -ExecutionPolicy Bypass -File scripts/run_perf_guard.ps1` is currently broken because `tools/bench-cli/src/bench/gui/interactions/step_patterns.rs` does not yet pass the required `snap_override` field to `SetWaveformSelectionRange`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` are parked while the performance lane is under review.

## Next tasks (ordered)

1. Wait for explicit user confirmation before beginning Phase 2 implementation from `tmp/perf_plan.md`.
2. When Phase 2 is approved, restore perf-guard compile parity first so fresh benchmark validation is available.
3. Keep `tmp/perf_plan.md`, `AGENTS.md`, `MEMORY.md`, and this TODO synchronized as the performance lane progresses.
4. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
