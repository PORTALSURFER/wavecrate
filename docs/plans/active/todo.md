# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-04T13:18:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- Cleanup architecture guardrails documented in `docs/plans/active/cleanup_architecture_note.md`.

## Next tasks (ordered)

1. Execute `tmp/perf_plan.md` item 7:
   cache/carry waveform upload blobs across draws to reduce image upload churn.
2. Execute `tmp/perf_plan.md` item 8:
   replace segment attribution proxy wiring with direct per-segment timings.
3. Execute `tmp/perf_plan.md` item 9:
   reduce map projection clone churn by retaining projected identity buffers.
4. Keep handoff docs synchronized at each milestone:
   update `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` in the same cycle.
