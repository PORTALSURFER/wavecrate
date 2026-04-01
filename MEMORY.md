# Agent Memory

Last Updated: 2026-04-01T23:55:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I am executing the refreshed evidence-driven improvement backlog for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth for the refreshed 2026-04-01 ROI-ranked backlog and execution record.
- Phase 2 is active. Items 1, 2, 3, and 4 are complete, and item 5 is next.
- Item 1 resolved the public waveform-shift contract mismatch by marking `begin_waveform_selection_shift` and `begin_waveform_edit_selection_shift` as runtime-internal catalog entries and by rejecting them from the public GUI runner instead of dispatching them into the unhandled `app_core` path.
- Item 1 is recorded in commit `b9e312ad` (`fix: reject runtime-internal waveform shift actions`).
- `docs/gui_test_platform.md` now documents that the action catalog is exhaustive but not every cataloged action is publicly dispatchable.
- Item 2 records handled state in GUI action traces, makes `ActionRecorded` require a handled event, and preserves `handled: false` in live artifacts instead of panicking on unhandled native actions.
- Item 2 is recorded in superproject commit `3f9a41cf` (`test: require handled GUI action traces`) and `vendor/radiant` commit `80cc200c` (`feat: expose last action handled state`).
- Item 3 routes focused keyboard tests and production runtime input through the same shared enter, escape, text-input, and hotkey helpers in `vendor/radiant`, removing the parallel test-only path.
- Item 3 is recorded in `vendor/radiant` commit `89c41e58` (`refactor: share native keyboard routing helpers`).
- Item 4 splits the `app_core` native browser and waveform dispatch hubs into smaller route-group modules and moves the remaining native-dispatch state mutations onto narrower legacy-controller seams.
- Item 4 is recorded in superproject commit `6dd61dc9` (`refactor: split app core native dispatch hubs`).
- The remaining backlog starts with the non-allowlisted production/runtime file-size debt, then oversized test hubs, `QUALITY_SCORE` drift, and shell-specific tooling drift.
- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passed during audit startup.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passed after item 1.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed after item 1.
- `cargo test app_core::native_bridge::tests::bridge_runtime::gui_test -- --test-threads=1` passed after item 2.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passed after item 2.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed after item 2.
- `cargo test --manifest-path vendor/radiant/Cargo.toml key_bindings -- --test-threads=1` passed after item 3.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passed after item 3.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed after item 3.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed after item 3.
- `cargo test app_core::controller::tests -- --test-threads=1` passed after item 4.
- `powershell -ExecutionPolicy Bypass -File scripts/check_migration_boundary.ps1` passed after item 4.
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1` passed after item 4.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed after item 4; one earlier harness-level abnormal exit on the same lane disappeared on rerun and did not reproduce.
- The truthful full-scan file-size budget is currently red and reports `23` non-allowlisted over-budget Rust files on the live tree.
- The refreshed cleanup-hotspot snapshot currently reports `30` total over-budget Rust files, `3` files with `dead_code` suppressions, and `3` files with `clippy::too_many_arguments` suppressions.
- The live audit currently records four open questions: which GUI actions are truly stable host API, stable action-id ownership across `app_core` and `radiant`, the exact `PlayFromStart` and `CommitVolumeSetting` contracts, and whether `src/selection/range.rs` should be treated as a cohesive exception or active file-size debt.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute item 5 from `tmp/improvement_audit_plan.md`: burn down the remaining non-allowlisted production/runtime file-size debt before touching explicitly allowlisted exceptions.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with the active Phase 2 execution state.
3. Continue the backlog in order once item 5 is complete.
4. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
5. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (refreshed on 2026-04-01; Phase 2 active, items 1, 2, 3, and 4 complete, item 5 next)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`
