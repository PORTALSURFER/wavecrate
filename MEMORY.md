# Agent Memory

Last Updated: 2026-03-31T10:37:52+02:00
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The active lane is Phase 2 execution of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth and was regenerated on `2026-03-30`.
- The workspace is not fully clean; several unrelated user-dirty files remain outside the audit lane, including `tools/gui-test-cli/src/main.rs`, multiple root-side controller files, and dirty `vendor/radiant/**` files outside the staged audit split.
- Item 1 is complete: the live migration-boundary failure at `HEAD` is fixed by routing the remaining `src/app_core/native_shell*` legacy state crossings back through migration-facing aliases, and `docs/gui_migration_parity.md` now records that boundary blocker honestly.
- Item 2 is complete in `772861f8`: compare-anchor is now documented and tested as a transient playback aid outside the meaningful undo/redo contract.
- Item 3 is complete and landed in `95364b39`.
- Item 4 is complete in local commit `d46bd589`: `MeaningfulUiSnapshot` and deferred async history restore coverage is broader and the history module is back under the file-size budget.
- Item 5 is complete in local commit `f50a0fe9`: unmatched `pending_wav_renames` rows now survive quick scans but are pruned on hard rescan.
- Items 6 and 7 are complete together in local commit `80b132fc`: `GuiScenarioStep::CaptureSnapshot` was removed from the supported scenario contract, the GUI runner was split into focused modules, and `src/gui_test/runner.rs` is no longer a live file-budget violation.
- Item 8 is complete in superproject commit `572beac8` plus `vendor/radiant` commit `bb734080`: the remaining `browser_sources` and `vendor/radiant` file-budget hotspots are split into focused modules, and the full file-size budget is green again.
- Validation is green again for the completed lane: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`, `cargo test -p radiant --lib --no-run`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` all pass.
- The completed audit commit stack is now pushed on `origin/next`.
- A small prerequisite test-harness fix was needed while validating item 3: `gui_test` unit coverage and the shell smoke pack now use deterministic named fixtures instead of the unstable persisted-startup `default` fixture where the assertions did not depend on it.
- The improvement-audit backlog is complete.
- The live full-scan file-size budget is green on this tree.
- Phase 2 is complete; the only remaining administrative step is pushing the validated commits.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Wait for the user to choose the next lane.
2. Keep `tmp/improvement_audit_plan.md` honest as the completed execution record for this lane.
3. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned once the push step is complete or a new lane starts.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-30; Phase 2 completed on 2026-03-31 and the document is now the completed execution record)
- Current broader hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

