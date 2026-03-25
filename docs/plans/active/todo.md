# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-26T00:28:36+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The refreshed evidence-driven improvement audit finished on 2026-03-25 for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth and execution log for that completed lane.
- `tmp/cleanup_audit_hotspots.md` was regenerated during this audit to refresh the broader file-size and test-gap evidence snapshot.
- `scripts/check_quality_score_drift.ps1` is green again after the vendor/radiant file-size guardrail fix, while the latest full-scan file-size budget still reports one unrelated live violation in `src/gui_test/runner.rs`.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the user to choose the next lane; the current audit backlog is complete.
2. Keep `tmp/improvement_audit_plan.md` available as the finished execution record and decision log for this lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the next active lane begins.
