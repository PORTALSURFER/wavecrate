# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-17T15:30:00+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The refreshed evidence-driven improvement audit backlog was executed sequentially on 2026-03-17.
- `tmp/improvement_audit_plan.md` is the source of truth for the completed execution record.
- The active follow-up is a dual-lane validation workflow: `scripts/ci_agent.*` for constrained agent environments and `scripts/ci_quick.*` / `scripts/ci_local.*` for broader human-run coverage.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Treat `tmp/improvement_audit_plan.md` as the completed execution record for the refreshed audit lane.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Use `scripts/ci_agent.ps1` as the default agent-side validation gate in this environment.
4. Treat `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.
5. Start a new audit or follow-on implementation lane only after explicit user direction.
6. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
