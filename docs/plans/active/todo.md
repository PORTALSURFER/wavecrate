# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-17T22:19:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the completed runtime performance backlog for the current live tree.
- `tmp/perf_plan.md` is the completed source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-17.
- Phase 2 is complete. Items 1-7 are done (`cec627fd`, `547a9c9b`, `140a8640`, `caf5d4cb`, `a384984a`; vendor `174aa295`, `eda74e21`, `b14e2423`).
- The perf guard now headlines the retained bridge path, captures Windows startup summaries, and the latest startup smoke in `tmp/perf_plan.md` records `first_present_ms = 1742.747`.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Keep `tmp/perf_plan.md` as the completion record for the 2026-04-17 runtime performance run.
2. Manual startup visual review remains the only recommended follow-up for the progressive reveal change in item 7.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
