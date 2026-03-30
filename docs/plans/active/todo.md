# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-30T12:25:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit of the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-03-30.
- Item 1 is complete; the live `app_core` migration-boundary failure at `HEAD` is fixed.
- Item 2 is the next ranked item, but it remains clarification-gated.
- The live full-scan file-size budget is still red on this tree and remains part of the refreshed audit backlog.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Record the item 1 commit in `tmp/improvement_audit_plan.md`.
2. Handle the item 2 clarification gate honestly, then continue with item 3 if it remains safe.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes or a clarification-gated item is resolved.
