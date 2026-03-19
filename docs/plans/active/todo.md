# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-19T12:57:05+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- A fresh evidence-driven improvement audit was rebuilt for the live tree on 2026-03-18 and is now in Phase 2 execution.
- `tmp/improvement_audit_plan.md` is the live execution tracker; items 1-3 are complete locally in commits `7e6baff1`, `c6b814d2`, and the next local source-move test commit, and item 4 is next.
- The active follow-up is a dual-lane validation workflow: `scripts/ci_agent.*` for constrained agent environments and `scripts/ci_quick.*` / `scripts/ci_local.*` for broader human-run coverage.
- Backlog item 4 remains the active push blocker because the current Windows validation wrappers still hit the pre-existing unhealthy `sccache` path here.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Implement item 4 from `tmp/improvement_audit_plan.md`: make the documented Windows agent-safe validation lane resilient when `sccache` is installed but unhealthy.
2. Keep `tmp/improvement_audit_plan.md` updated after each completed item with status, assumptions, validation, and commit metadata.
3. Treat backlog item 4 as the active blocker for push until `scripts/devcheck.ps1` and `scripts/ci_agent.ps1` stop falling into the unhealthy `sccache` wrapper path.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Treat `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.
6. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
