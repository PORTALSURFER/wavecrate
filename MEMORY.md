# Agent Memory

Last Updated: 2026-03-12T10:22:32Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- I have completed the runtime performance backlog in `tmp/perf_plan.md` through item 11.
- I have completed the previous cleanup lanes recorded in older `tmp/cleanup_plan.md` revisions.
- I have now opened a fresh cleanup audit pass and rebuilt `tmp/cleanup_plan.md`.
- The refreshed cleanup plan is now in Phase 2 execution.
- Cleanup item 1 is complete in `vendor/radiant` commit `e0ce0710`.
- Cleanup item 2 is complete in `vendor/radiant` commit `c1d68d7a`.
- Cleanup item 3 is complete in `vendor/radiant` commit `bcaaef6d`.
- Cleanup item 4 is complete in `vendor/radiant` commit `412426a9`.
- Cleanup item 5 is complete in main repo commit `f8dbd240`.
- Cleanup item 6 is complete in main repo commit `072fb0ca`.
- Cleanup item 7 is complete in main repo commit `17d911b2`.
- Cleanup item 8 is complete in main repo commit `ae5e1715`.
- Cleanup item 9 is complete in main repo commit `803f9d2e`.
- Cleanup item 10 is complete in main repo commit `50f4da56`.
- Cleanup item 11 is complete in main repo commit `fee4901d`.
- Cleanup item 12 is complete in main repo commit `d1776d5e`.
- I have implemented the first GUI test platform foundation slice on `next`.
- The new GUI platform source docs are `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md`.
- The new GUI platform adds a host-side action catalog, native-shell automation snapshots, deterministic GUI test-mode artifact plumbing, a `gui-test-cli`, and PowerShell GUI test loop scripts.
- The new GUI test loop currently validates through `scripts/run_gui_contract.ps1`, `scripts/run_gui_suite.ps1`, and `scripts/ci_quick.ps1`.
- There are 4 remaining cleanup items in strict ROI order.
- Cleanup item 13 is next.
- The current cleanup source of truth is `tmp/cleanup_plan.md`.
- The perf source of truth remains `docs/plans/active/runtime_performance_exec_plan.md` and stays dormant unless a separate perf lane is reopened.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.
- The current `vendor/radiant` checkout still has unported local work on `codex/radiant-ci-cleanup`; it cannot switch to `next` until that work is stashed, committed, or ported.

## Immediate Next Actions

1. Continue cleanup at item 13 in `tmp/cleanup_plan.md`.
2. After each completed cleanup item, rerun validation and update `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md`.
3. Commit and push each cleanup milestone after quick CI is green.
4. Use `docs/gui_test_platform.md` and `docs/plans/active/gui_test_platform_exec_plan.md` as the source of truth for the GUI automation/test rollout.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`

