# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-17T19:57:30+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the follow-up runtime performance audit for the current live tree.
- `tmp/perf_plan.md` is the active source of truth for the rebuilt 2026-04-17 ROI-ranked backlog.
- Phase 1 is complete. The plan now contains eight pending items plus fresh guard, startup-profile, and waveform-preview A/B evidence.
- Wait for explicit user approval before any Phase 2 implementation, commit, or push work.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Present the ordered backlog from `tmp/perf_plan.md` to the user and ask for Phase 2 approval.
2. If the user confirms, implement items strictly in plan order and update `tmp/perf_plan.md` after each item.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
