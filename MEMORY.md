# Agent Memory

Last Updated: 2026-03-29T23:12:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth.
- The previous 2026-03-25 completed execution record at that path has been replaced by a new 2026-03-29 ROI-ranked backlog and live execution record for the current tree.
- The current tree is not a clean baseline; the worktree is already dirty and the audit treats those edits as read-only context.
- The live full-scan file-size budget is red again on this tree; `scripts/check_file_size_budget.ps1 -All` reported current repo and `vendor/radiant` violations during the audit.
- Item 1 fixed `scripts/check_quality_score_drift.ps1` so benign nested Git CRLF warnings no longer abort guardrail classification on Windows.
- Item 2 fixed the cleanup-hotspot audit scripts so `tmp/cleanup_audit_hotspots.md` now includes `vendor/radiant` and exposes the live runtime hotspot cluster again.
- Item 3 refreshed `docs/QUALITY_SCORE.md` so the scorecard now describes the live observed dirty workspace, including the degraded file-size posture and suppression counts.
- Item 7 added direct mutation-invariant coverage for `entry_mutation`, including DB metadata rewrite, cache lookup rewrite, browser focus path rewrite, and compare-anchor path updates during rename/move flows.
- Item 10 added direct `gui-test-cli` command-surface coverage by extracting a parse seam and testing argument handling for every supported top-level command without spawning the full app.
- Item 11 added direct installer-entry coverage by extracting a tiny command-selection/dispatch seam and testing uninstall, dry-run, and default UI launch behavior without starting the installer UI.
- Item 12 split the targeted `vendor/radiant` chrome-sidebar hotspot cluster into focused hit-testing and frame-build submodules, removing both cluster entry files from the live file-size violation list.
- Item 4 still needs compare-anchor product clarification, item 5 is blocked on that decision, and item 6 still needs pending-rename lifecycle clarification.
- Item 8 still needs GUI scenario capture-step clarification, and item 9 is blocked on that contract.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for user clarification on item 4, item 6, or item 8 before resuming the remaining blocked audit backlog, or wait for the user to choose a new lane.
2. Keep `tmp/improvement_audit_plan.md`, `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned around the paused clarification-gated state.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Use `scripts/ci_agent.ps1` for agent-side validation in this constrained Windows environment, and treat `scripts/ci_quick.ps1` / `scripts/ci_local.ps1` as broader confirmation lanes when `cargo-nextest.exe` is allowed.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-29; Phase 2 safe executable items are complete, and the remaining backlog is clarification-gated)
- Current broader hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
