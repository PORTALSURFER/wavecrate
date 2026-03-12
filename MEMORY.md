# Agent Memory

Last Updated: 2026-03-12T11:39:30Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` is at commit `e3208ca6` and matches `origin/next`.
- `C:\dev\sempal\vendor\radiant` is on `next` at commit `180865c8` and matches `origin/next`.
- `vendor/radiant` `next` now contains the code state that previously lived on `origin/codex/browser-wheel-scrollbar`.
- I keep the previous remote `radiant/next` state locally on `codex/radiant-next-backup-20260312`.
- I have completed the runtime performance backlog in `tmp/perf_plan.md` through item 11.
- I have completed the previous cleanup lanes recorded in older `tmp/cleanup_plan.md` revisions.
- I am running a fresh cleanup audit pass and have rebuilt `tmp/cleanup_plan.md` from the current codebase.
- This refreshed cleanup plan is still in Phase 1 state; no items from this new pass are implemented yet.
- Phase 2 must not start until the user explicitly confirms the ordered backlog.
- I have implemented the first GUI test platform foundation slice on `next`.
- The new GUI platform source docs are `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md`.
- The new GUI platform adds a host-side action catalog, native-shell automation snapshots, deterministic GUI test-mode artifact plumbing, a `gui-test-cli`, and PowerShell GUI test loop scripts.
- The new GUI test loop currently validates through `scripts/run_gui_contract.ps1`, `scripts/run_gui_suite.ps1`, and `scripts/ci_quick.ps1`.
- The current cleanup source of truth is `tmp/cleanup_plan.md`.
- The perf source of truth remains `docs/plans/active/runtime_performance_exec_plan.md` and stays dormant unless a separate perf lane is reopened.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.

## Immediate Next Actions

1. Review the rebuilt `tmp/cleanup_plan.md` backlog with the user and wait for explicit confirmation before Phase 2.
2. If approved, execute cleanup strictly in plan order and update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` after each completed item.
3. Commit and push each cleanup milestone after quick CI is green.
4. Use `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md` as the source of truth for the GUI automation/test rollout.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`

