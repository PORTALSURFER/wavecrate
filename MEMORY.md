# Agent Memory

Last Updated: 2026-03-12T20:53:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` is at commit `43afec47` and matches `origin/next` before the current superproject bookkeeping commit.
- `C:\dev\sempal\vendor\radiant` is on `next` at commit `711f159a` and matches `origin/next`.
- `vendor/radiant` `next` now contains the code state that previously lived on `origin/codex/browser-wheel-scrollbar`.
- I keep the previous remote `radiant/next` state locally on `codex/radiant-next-backup-20260312`.
- I have completed the runtime performance backlog in `tmp/perf_plan.md` through item 11.
- I have completed the previous cleanup lanes recorded in older `tmp/cleanup_plan.md` revisions.
- I am running a fresh cleanup audit pass and have rebuilt `tmp/cleanup_plan.md` from the current codebase.
- Phase 2 is active.
- Cleanup item 1 is complete in `vendor/radiant` commit `f8063a2b`.
- Item 1 makes `radiant` host-neutral by replacing Sempal-specific default titles, startup placeholder branding, automation root labels, and radiant-only fixture names/docs with `Radiant` defaults while still letting the host pass explicit branding.
- Cleanup item 2 is complete in `vendor/radiant` commit `f2d98345` plus the matching superproject allowlist/metadata refresh.
- Item 2 removes stale cleanup references from crate roots and rebuilds `docs/file_size_budget_allowlist.txt` so it matches the actual current >400-line debt.
- Cleanup item 3 is complete in `vendor/radiant` commit `2cc5c0f4`.
- Item 3 removes duplicated waveform tempo parsing and browser/BPM text-field pointer-selection helpers by routing both shell/runtime paths through one shared parser and one shared native-vello text-input helper flow.
- Cleanup item 4 is complete in `vendor/radiant` commit `2a819231`.
- Item 4 replaces the monolithic native-shell hit-testing file with focused hover, browser, chrome, waveform, and map modules while preserving the existing state API and test surface through a compatibility facade.
- Cleanup item 5 is complete in `vendor/radiant` commit `d1f5ba4a`.
- Item 5 replaces the monolithic native-vello render core with focused invalidation, scene, present, and redraw-profiling modules while preserving the runtime API surface and quick-CI behavior.
- Cleanup item 6 is complete in `vendor/radiant` commit `711f159a`.
- Item 6 centralizes native-vello pointer-session and drag/text-input ownership behind runner transition helpers so runtime events/input stop mutating drag state ad hoc across files.
- Sempal commit `c392908d` stabilizes the quick-CI validation lane by fixing the async decode prefill completion race and tightening the decode-heartbeat timing path/test.
- Cleanup item 7 is complete in the current superproject worktree.
- Item 7 replaces the monolithic projection-key builder file with a focused `projection_key/` module tree so browser, status, map, waveform, and non-segment invalidation keys each live in their own module while preserving the existing native-bridge builder API.
- Cleanup item 8 is complete in the current superproject worktree.
- Item 8 replaces the monolithic clipboard source import worker with a focused `source_job/` module tree so prepare, stage, commit, finalize, and cleanup responsibilities are explicit while preserving the staged-copy recovery semantics.
- Item 9 is next in `tmp/cleanup_plan.md`.
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

1. Continue cleanup at item 9 in `tmp/cleanup_plan.md`.
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

