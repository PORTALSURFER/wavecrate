# Agent Memory

Last Updated: 2026-03-25T19:09:04Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is sequential Phase 2 execution of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- The audit was rebuilt on commit `8056af85` on 2026-03-25, Phase 2 execution is now in progress, and item 9 is clarified so direct tagging to `KEEP_3` must also lock.
- `tmp/cleanup_audit_hotspots.md` was regenerated during this audit and now reflects the broader 2026-03-25 hotspot snapshot for the live tree.
- The repo-entry docs are not fully aligned: `docs/README.md` and `docs/plans/active/gui_test_platform_exec_plan.md` still contain stale status text that the refreshed audit plan calls out explicitly.
- The full quality-score drift check is green again after the execution lane restored the enforced file-size budget and refreshed `docs/QUALITY_SCORE.md`.
- The guardrail-scope file-size budget now passes, with two explicitly documented cohesive exceptions in `docs/file_size_budget_allowlist.txt`: `src/app_core/actions/catalog/kinds.rs` and `src/app/controller/playback/transport/selection.rs`.
- The broader cleanup hotspot snapshot currently reports 11 over-budget Rust files across the wider scan scope.
- The current top ROI themes are: a false-negative guardrail around `vendor/radiant` file-size checks, stale handoff/release docs, high-value recovery/test gaps around drag-drop file ops and GUI contract helpers, and a still-oversized staged audio-loader module.
- The dual-lane Windows validation workflow is still the same: `scripts/ci_agent.ps1` is the reliable constrained-environment lane, while `scripts/ci_quick.ps1` remains the broader integrated confirmation lane when `cargo-nextest.exe` is allowed.
- The PowerShell validation wrappers still need to preserve the direct-`rustc` plus `tmp/agent_temp` fallback path whenever inherited `sccache` or the default temp dir is unusable.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute `tmp/improvement_audit_plan.md` sequentially in ranked order and keep the file updated after each completed item.
2. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `docs/plans/active/todo.md`, `docs/plans/index.md`, and this file aligned around the active execution lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Use `scripts/ci_agent.ps1` for agent-side validation in this constrained Windows environment, and treat `scripts/ci_quick.ps1` / `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.

## Work Notes

- Active audit backlog: `tmp/improvement_audit_plan.md` (Phase 2 execution started on 2026-03-25)
- Current cleanup hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

