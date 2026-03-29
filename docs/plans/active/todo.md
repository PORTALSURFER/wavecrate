# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-29T22:48:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog and execution record rebuilt on 2026-03-29.
- Items 1-3, 7, and 10-12 are complete; items 4-6 and 8-9 are clarification-gated or blocked, and there is no remaining safe executable item until one of those product questions is resolved.
- The live full-scan file-size budget is red again on this tree and is now part of the refreshed audit backlog.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for user clarification on item 4, item 6, or item 8 before resuming the remaining blocked audit backlog, or wait for the user to choose a new lane.
2. Keep `tmp/improvement_audit_plan.md` available as the active execution record and decision log for this paused lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes or a blocked clarification item is resolved.
