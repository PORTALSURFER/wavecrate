# Agent Memory

Last Updated: 2026-03-13T08:34:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` is at commit `aff5d14b` on `next` and matches `origin/next`.
- `C:\dev\sempal\vendor\radiant` is on `next` at commit `1771ad6b` and matches `origin/next`.
- The browser sample-list autoscroll threshold fix is complete: selecting interior rows no longer recenters the viewport, and scrolling now begins only when focus enters the top or bottom three visible rows.
- The vendor-side regression coverage for that behavior lives in `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs`.
- The desktop regression coverage for that behavior lives in `src/gui_test/aiv/packs/cases.rs` and passes via `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_suite.ps1 -PackName desktop-regression -CaseFilter browser_interior_click_keeps_viewport`.
- `tmp/cleanup_plan.md` remains the parked cleanup source of truth.
- Cleanup Phase 1 is complete, and Phase 2 is still waiting on explicit user confirmation.
- I have implemented the first GUI test platform foundation slice on `next`.
- The new GUI platform source docs are `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md`.
- The new GUI platform adds a host-side action catalog, native-shell automation snapshots, deterministic GUI test-mode artifact plumbing, a `gui-test-cli`, and PowerShell GUI test loop scripts.
- The new GUI test loop currently validates through `scripts/run_gui_contract.ps1`, `scripts/run_gui_suite.ps1`, and `scripts/ci_quick.ps1`.
- The perf source of truth remains `docs/plans/active/runtime_performance_exec_plan.md` and stays dormant unless a separate perf lane is reopened.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.

## Immediate Next Actions

1. Keep the cleanup backlog dormant unless the user explicitly asks to resume it.
2. For future browser list interaction changes, rerun the targeted AIV pack cases and the PowerShell validation wrappers.
3. Update the handoff docs again when the active lane changes.
4. Commit and push each coherent milestone after quick CI is green.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`
