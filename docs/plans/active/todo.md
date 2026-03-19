# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-19T16:15:00+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- The refreshed evidence-driven improvement audit rebuilt on 2026-03-18 has completed its Phase 2 backlog implementation.
- `tmp/improvement_audit_plan.md` is the live execution record for all 10 backlog items; items 1-9 have recorded commits, and item 10 is complete locally pending its final commit/push closeout.
- The active follow-up is a dual-lane validation workflow: `scripts/ci_agent.*` for constrained agent environments and `scripts/ci_quick.*` / `scripts/ci_local.*` for broader human-run coverage.
- The Windows PowerShell validation lane is green again after the wrapper fallback now bypasses unhealthy inherited `sccache` and unwritable default temp paths.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Keep `tmp/improvement_audit_plan.md` synchronized with the final item-10 commit metadata after commit/push closeout.
2. Treat `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as broader user-run confirmation lanes when requested and when `cargo-nextest.exe` is allowed.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
