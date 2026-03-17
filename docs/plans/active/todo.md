# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-17T23:35:00+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- A fresh evidence-driven improvement audit has been rebuilt for the live tree on 2026-03-17.
- `tmp/improvement_audit_plan.md` is the source of truth for the current Phase 1 backlog and is waiting for explicit implementation confirmation.
- The active follow-up is a dual-lane validation workflow: `scripts/ci_agent.*` for constrained agent environments and `scripts/ci_quick.*` / `scripts/ci_local.*` for broader human-run coverage.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for explicit user confirmation before implementing any ranked backlog item from `tmp/improvement_audit_plan.md`.
2. Keep `tmp/improvement_audit_plan.md` as the Phase 1 source of truth for the current audit lane.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Use `scripts/ci_agent.ps1` as the default agent-side validation gate in this environment.
5. Treat `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.
6. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
