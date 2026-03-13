# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-03-13T08:34:00Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- GUI/browser interaction stability and desktop regression coverage maintenance.
- The browser autoscroll threshold fix and its targeted desktop AIV coverage are complete on `next`.
- Perf Phase 2 items 1-11 in `tmp/perf_plan.md` remain complete.
- The cleanup backlog in `tmp/cleanup_plan.md` is parked after Phase 1 and is waiting on explicit user confirmation before any Phase 2 implementation.

## Next tasks (ordered)

1. For future browser/sample-list interaction work, rerun `scripts/run_gui_aiv_suite.ps1 -PackName desktop-regression -CaseFilter browser_interior_click_keeps_viewport`.
2. Keep `tmp/cleanup_plan.md` dormant unless the user explicitly reopens cleanup Phase 2.
3. After the active lane changes, sync `AGENTS.md`, `MEMORY.md`, and this file.
4. Keep using the PowerShell wrappers (`devcheck.ps1`, `ci_quick.ps1`) as the validation gate on Windows.
