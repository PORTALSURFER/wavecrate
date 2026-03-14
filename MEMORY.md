# Agent Memory

Last Updated: 2026-03-14T10:47:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` are both on local `next`.
- The working tree is clean after completing the improvement-audit execution lane and syncing the handoff docs.
- The improvement audit lane is complete.
- The current source of truth is `tmp/improvement_audit_plan.md`.
- `tmp/improvement_audit_plan.md` was rebuilt on `2026-03-14` as a fresh evidence-driven ROI-ranked backlog for the current codebase and now serves as the execution record.
- Backlog item 1 is complete and pushed as `a1bdd698`: the Windows migration-boundary guard now prints actionable violations, correctly skips test-only paths on Windows-style separators, and is covered by script guardrail fixtures.
- Backlog item 2 is complete and pushed as `7a362804`: the async browser search worker now matches the sync pipeline's fuzzy-score ordering for query results, and a dedicated parity test compares worker output against the controller's sync visible rows.
- Backlog item 3 is complete and pushed as `888b47a2`: controller tests can now force the runtime-default async browser-search dispatch path and deterministically apply background search results to verify busy-state and stale-request behavior.
- Backlog item 4 is complete and pushed as `b232cfec`: `app_core::controller` and `app_core::controller::waveform_actions` no longer import `crate::app::` directly, and the migration-boundary scripts now enforce `app_core::app_api` as the only non-test crossing.
- Backlog item 5 is complete and pushed as `fe44b501`: the file-size debt ledger and quality docs now reflect the live guardrail scope and the current cleanup hotspot snapshot.
- Backlog item 6 is complete and pushed as `4e5d82e7`: the GUI action catalog is now split into focused modules without changing the public `app_core::actions` contract surface.
- Backlog item 7 is complete and pushed as `905919e1`: the installer UI bridge now separates workflow state, UI projection, and runtime wrapper code, with direct tests for retry recovery and location-step projection.
- Backlog item 8 is complete and pushed as `79604cbd`: playback tagging now shares one selection/undo helper path across `tag_selected` and `adjust_selected_rating`, with an added regression test for undo refocus under filtered rating changes.
- Backlog item 9 is complete across `bf0abada`, `6c70247d`, and `fe13990d`: the Windows external drag path now has direct payload-format tests, a split `external_drag/` module tree, and no lingering legacy single-file module.
- Backlog item 10 is complete and pushed as `07639512`: the remaining large regression catalogs are split into behavior-focused module trees under `browser_core/`, `focus_random/`, `source_db_mod_tests/`, and `analysis_jobs/enqueue/tests/`.
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

1. Wait for the next user-directed lane; do not reopen `tmp/improvement_audit_plan.md` unless the user asks for follow-up audit work.
2. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` dormant unless the user explicitly reopens those lanes.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and this file aligned when the active lane changes.
4. If broader CI parity is requested again, note that `scripts/ci_local.ps1` still stops later on the pre-existing unrelated `vendor/radiant` layout test.

## Work Notes

- Active improvement audit backlog: `tmp/improvement_audit_plan.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`





