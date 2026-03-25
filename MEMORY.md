# Agent Memory

Last Updated: 2026-03-25T23:28:43Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is the completed evidence-driven improvement audit execution record for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- The audit was rebuilt on commit `8056af85` on 2026-03-25, and all 10 ranked Phase 2 items are now completed with execution history recorded in `tmp/improvement_audit_plan.md`.
- `tmp/cleanup_audit_hotspots.md` was regenerated during this audit and now reflects the broader 2026-03-25 hotspot snapshot for the live tree.
- The repo-entry docs and plan index are aligned around the completed audit lane.
- The full quality-score drift check is green again after the execution lane restored the enforced vendor/radiant file-size guardrail and refreshed `docs/QUALITY_SCORE.md`.
- The latest full-scan file-size budget still reports one unrelated live violation in `src/gui_test/runner.rs` at 410 lines.
- The broader cleanup hotspot snapshot remains reference material rather than the live source of truth for the completed audit lane.
- The dual-lane Windows validation workflow is still the same: `scripts/ci_agent.ps1` is the reliable constrained-environment lane, while `scripts/ci_quick.ps1` remains the broader integrated confirmation lane when `cargo-nextest.exe` is allowed.
- The PowerShell validation wrappers still need to preserve the direct-`rustc` plus `tmp/agent_temp` fallback path whenever inherited `sccache` or the default temp dir is unusable.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for the user to choose the next lane; the improvement audit backlog is complete.
2. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `docs/plans/active/todo.md`, `docs/plans/index.md`, and this file aligned around the next active lane.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Use `scripts/ci_agent.ps1` for agent-side validation in this constrained Windows environment, and treat `scripts/ci_quick.ps1` / `scripts/ci_local.ps1` as broader user-run confirmation lanes when `cargo-nextest.exe` is allowed.

## Work Notes

- Active audit execution record: `tmp/improvement_audit_plan.md` (all 10 ranked items completed on 2026-03-25)
- Current cleanup hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

