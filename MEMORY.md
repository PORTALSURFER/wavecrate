# Agent Memory

Last Updated: 2026-04-02T23:33:00+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 repo-wide ROI-ranked backlog for the current tree.
- Phase 2 is now active for the 2026-04-02 improvement audit backlog.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; that snapshot is the current supporting hotspot picture behind the new ranked plan.
- Item 1 from `tmp/improvement_audit_plan.md` is implemented locally and validated:
  - playback-age filter invalidation in `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` now rolls at the next relevant filter-boundary change instead of every second
  - the browser pipeline tests now cover the week-rollover and month-only rollover cache behavior
- The next ranked items are:
  - adding broader direct coverage for the browser visible-row pipeline branches
  - expanding automation action-id parity checks beyond representative nodes
  - consolidating browser focus/selection ownership between `selection_ops.rs` and `focus_navigation.rs`
  - splitting the `vendor/radiant` hotkey catalog and the oversized `native_vello` gesture test hubs
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Commit and push the completed item-1 playback-age invalidation fix with its plan update.
2. Continue with item 2 from `tmp/improvement_audit_plan.md` in ranked order.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with this audit lane.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (Phase 2 active; item 1 complete locally and validated)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
