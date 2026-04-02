# Agent Memory

Last Updated: 2026-04-02T23:12:20+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I have refreshed the evidence-driven improvement audit for the current live tree and written the new Phase 1 plan to `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` is now the source of truth for the 2026-04-02 repo-wide ROI-ranked backlog for the current tree.
- Phase 1 is complete and no implementation has started yet; explicit user confirmation is still required before any Phase 2 work begins.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passes on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md` on `2026-04-02`; that snapshot is the current supporting hotspot picture behind the new ranked plan.
- The highest-ROI backlog in the new audit is concentrated in GUI contract integrity, keyboard contract maintainability, runtime test structure, and one newly justified transport cleanup:
  - expanding automation action-id parity checks beyond representative nodes
  - splitting the `vendor/radiant` hotkey catalog into scope-owned binding slices while preserving the flat public surface
  - decomposing the oversized `native_vello` runtime gesture test hubs by interaction family
  - revisiting `src/app/controller/playback/transport/selection.rs` now that its allowlist note appears satisfied
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the active Phase 1 backlog for this lane.
2. Wait for explicit user confirmation before starting any Phase 2 implementation.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with this audit lane.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (refreshed on 2026-04-02; Phase 1 complete and awaiting confirmation)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
