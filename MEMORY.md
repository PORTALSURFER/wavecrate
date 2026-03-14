# Agent Memory

Last Updated: 2026-03-14T10:28:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are both on local `next`.
- The working tree contains the current improvement-audit execution changes for backlog item 6 plus synchronized plan docs.
- The improvement audit lane is active in Phase 2.
- The current source of truth is `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-14` as a fresh evidence-driven ROI-ranked backlog for the current codebase and now serves as the execution record.
- Backlog item 1 is complete and pushed as `a1bdd698`: the Windows migration-boundary guard now prints actionable violations, correctly skips test-only paths on Windows-style separators, and is covered by script guardrail fixtures.
- Backlog item 2 is complete and pushed as `7a362804`: the async browser search worker now matches the sync pipeline's fuzzy-score ordering for query results, and a dedicated parity test compares worker output against the controller's sync visible rows.
- Backlog item 3 is complete and pushed as `888b47a2`: controller tests can now force the runtime-default async browser-search dispatch path and deterministically apply background search results to verify busy-state and stale-request behavior.
- Backlog item 4 is complete and pushed as `b232cfec`: `app_core::controller` and `app_core::controller::waveform_actions` no longer import `crate::app::` directly, and the migration-boundary scripts now enforce `app_core::app_api` as the only non-test crossing.
- Backlog item 5 is complete and pushed as `fe44b501`: the file-size debt ledger and quality docs now reflect the live guardrail scope and the current cleanup hotspot snapshot.
- Backlog item 6 is complete in the working tree and ready to commit: the canonical GUI action catalog now lives under focused `catalog/` modules for action kinds, coverage metadata, and entry lookup while preserving the existing `app_core::actions` surface and contract tests.
- `scripts/ci_quick.ps1` is green after the item-1 change.
- `scripts/ci_local.ps1` now gets past the migration-boundary gate and currently fails later on the pre-existing unrelated `vendor/radiant` test `gui::native_shell::layout_adapter::controls::controls_tests::toolbar_search_field_uses_ratio_width_inside_full_host`.
- The earlier GUI/browser interaction fixes and desktop AIV coverage remain part of the current repository state and are background context, not the active execution lane.
- `tmp/cleanup_plan.md` remains parked after Phase 1 and still requires explicit confirmation before any cleanup implementation.
- `tmp/perf_plan.md` remains parked after its earlier performance work and should stay dormant unless the user reopens that lane.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.
- Future development should use the `next` branch in both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- The branch policy is now enforced by `scripts/check_next_branch.ps1`/`.sh` plus git hooks installed by `scripts/install_agent_preflight_hooks.sh`.

## Immediate Next Actions

1. Commit and push backlog item 6 from `tmp/improvement_audit_plan.md`, then start item 7.
2. Keep `tmp/improvement_audit_plan.md` updated after each completed item with date, commit hash, assumptions, and validation results.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned with the active lane summary.

## Work Notes

- Active improvement audit backlog: `tmp/improvement_audit_plan.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`





