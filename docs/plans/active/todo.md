# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-03-25T18:25:26+01:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/improvement_audit_plan.md`.

## Current lane

- The refreshed evidence-driven improvement audit execution completed on 2026-03-25 for the current live tree.
- `tmp/improvement_audit_plan.md` is the live source of truth and final execution record for that completed lane.
- `tmp/cleanup_audit_hotspots.md` was regenerated during this audit to refresh the broader file-size and test-gap evidence snapshot.
- The full file-size guardrail is green again, with two documented cohesive exceptions in `docs/file_size_budget_allowlist.txt`, and `scripts/check_quality_score_drift.ps1` now passes.
- The cleanup backlog in `tmp/cleanup_plan.md` and the perf backlog in `tmp/perf_plan.md` both remain parked.

## Next tasks (ordered)

1. Wait for the user to choose the next lane now that the audit backlog is complete.
2. Keep `tmp/improvement_audit_plan.md` as the live audit execution record and decision log for the current tree.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
5. Keep `AGENTS.md`, `MEMORY.md`, this file, and `docs/plans/index.md` synchronized when the active lane changes.
