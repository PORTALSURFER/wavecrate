# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-31T10:37:52+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit of the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth for the ranked backlog rebuilt on 2026-03-30.
- Items 1-8 are complete locally.
- The remaining file-size debt is closed in superproject commit `572beac8` and `vendor/radiant` commit `bb734080`.
- `scripts/check_file_size_budget.ps1 -All`, `cargo test -p radiant --lib --no-run`, `scripts/devcheck.ps1`, and `scripts/ci_agent.ps1` are green.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the user to choose the next lane.
2. Keep `tmp/improvement_audit_plan.md` updated as the completed execution record for this lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the push step completes or the active lane changes.
