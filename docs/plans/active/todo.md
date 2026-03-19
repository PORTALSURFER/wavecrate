# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-19T14:17:28+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- A fresh evidence-driven improvement audit was rebuilt for the live tree on 2026-03-18 and is now in Phase 2 execution.
- `tmp/improvement_audit_plan.md` is the live execution tracker; items 1-8 are complete locally, with commits recorded for items 1-7 in `7e6baff1`, `c6b814d2`, `91e5c30e`, `d0685aad`, `8b0637d7`, `78430bfa`, and `6b24829d`, and item 9 is next.
- The active follow-up is a dual-lane validation workflow: `scripts/ci_agent.*` for constrained agent environments and `scripts/ci_quick.*` / `scripts/ci_local.*` for broader human-run coverage.
- The Windows PowerShell validation lane is green again after the wrapper fallback now bypasses unhealthy inherited `sccache` and unwritable default temp paths.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Implement item 9 from `tmp/improvement_audit_plan.md`: refresh stale architecture and planning docs that no longer match the live tree.
2. Keep `tmp/improvement_audit_plan.md` updated after each completed item with status, assumptions, validation, and commit metadata.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Treat `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.
6. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
