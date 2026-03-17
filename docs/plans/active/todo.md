# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-17T09:45:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The older approved backlog and the newer remote audit refresh were merged on 2026-03-17.
- `tmp/improvement_audit_plan.md` is the source of truth for the merged ROI-ranked execution backlog.
- Phase 2 is in progress against that merged plan.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Finish the merged item 1 work in `tmp/improvement_audit_plan.md` so the file-size and quality-score guardrails recover.
2. Continue the merged backlog sequentially from `scan.rs`, `selection.rs`, audio-options tests/refactor, `options_panel.rs`, and `profiling.rs`.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `MEMORY.md`, this file, and `tmp/improvement_audit_plan.md` synchronized as merged backlog items complete.
