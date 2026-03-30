# Agent Memory

Last Updated: 2026-03-30T13:30:46+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth and was regenerated on `2026-03-30`.
- The workspace is clean in the superproject at audit time.
- Item 1 is complete: the live migration-boundary failure at `HEAD` is fixed by routing the remaining `src/app_core/native_shell*` legacy state crossings back through migration-facing aliases, and `docs/gui_migration_parity.md` now records that boundary blocker honestly.
- Item 3 is complete and landed in `95364b39`.
- Item 8 burned down the clean root-side file-size debt through focused splits in analysis/audio helpers, controller runtime/audio-loading helpers, and oversized test modules. The remaining file-budget failures are now limited to the dirty `src/app_core/controller/tests/browser_sources.rs`, the clarification-gated `src/gui_test/runner.rs`, and dirty `vendor/radiant/**` files.
- Repo-wide validation is green again: `cargo test -p sempal --lib -- --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` all pass after the item 8 safe-scope splits.
- A small prerequisite test-harness fix was needed while validating item 3: `gui_test` unit coverage and the shell smoke pack now use deterministic named fixtures instead of the unstable persisted-startup `default` fixture where the assertions did not depend on it.
- The remaining top backlog still centers on compare-anchor undo ambiguity, pending-rename lifecycle clarification, the misleading `GuiScenarioStep::CaptureSnapshot` contract, and the blocked remainder of the file-size debt.
- The live full-scan file-size budget is still red on this tree, but only for 16 blocked/dirty-scope files instead of the earlier 29 active-scope failures.
- Phase 2 is paused. Item 2 still needs compare-anchor clarification before it can be resolved honestly.
- Clarification is still required for compare-anchor undo semantics, pending-rename retention, and the GUI scenario capture-step contract.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for user clarification on item 2, item 5, or item 6 before resuming the blocked backlog, or wait for direction on whether the dirty `src/app_core/controller/tests/browser_sources.rs` / `vendor/radiant/**` files can be touched.
2. Keep `tmp/improvement_audit_plan.md` honest about the item 8 blockers and the landed root-side burn-down work.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with the live paused state recorded in `tmp/improvement_audit_plan.md`.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-30; Phase 2 paused after the safe executable backlog was burned down; remaining work is clarification-gated or blocked by unrelated dirty files)
- Current broader hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

