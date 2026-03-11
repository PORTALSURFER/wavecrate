# Agent Memory

Last Updated: 2026-03-11T15:33:24Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed the Phase 2 runtime performance execution backlog in `tmp/perf_plan.md`.
- Perf items 1-11 are complete, including hot native input layout borrowing in vendor commit `fd542453` and root perf attribution work in commit `f239b03b`.
- The active source of truth for any follow-up perf work remains `docs/plans/active/runtime_performance_exec_plan.md`.
- I have completed the earlier 30-item cleanup backlog.
- I have refreshed `tmp/cleanup_plan.md` against the current `next` head with a new second-pass 15-item strict ROI-ranked cleanup backlog.
- I have completed cleanup item 14 by splitting `src/app/controller/playback/waveform_actions.rs` into focused waveform-action submodules while keeping the public controller surface stable.
- I have completed cleanup item 15 by splitting `src/app_core/native_bridge.rs` into focused batching, reducer, and projection-runtime modules and by splitting the native-bridge test surface into queue, bridge-runtime, and projection-cache modules.
- I have completed cleanup item 16 by splitting `src/sample_sources/db/schema.rs` into focused schema DDL and migration modules and by adding legacy migration fixtures for `wav_files`, `analysis_jobs`, and `samples`.
- I have completed cleanup item 17 by routing one-shot source DB mutations through `SourceWriteBatch`, consolidating wav upsert/update helpers, and adding wrapper-vs-batch parity coverage.
- I have completed cleanup item 18 by moving controller performance, status, undo, and small job/playback helpers out of `src/app/controller.rs`, leaving the root controller focused on construction and top-level accessors.
- I have completed cleanup item 19 by turning `src/app/controller/library/wavs/similar/resolve.rs` into focused repository, reranking, and analysis-enqueue modules while keeping the similarity query surface stable.
- I have completed cleanup item 20 by splitting `vendor/radiant` native-shell retained-state sync/cache helpers and frame-build status-bar helpers into focused modules while preserving the retained shell behavior and native-shell tests.
- I have completed cleanup item 21 by splitting `vendor/radiant` native Vello waveform input routing into focused geometry, handle, and wheel modules and by extracting the immediate cursor/drag runtime state machine into `runtime_input.rs`.
- I have completed cleanup item 22 by splitting `vendor/radiant/src/app/mod.rs` into focused public contract modules for actions, browser/map models, bridge traits, dirty-segment bookkeeping, motion projection, shell overlays, source/sidebar models, and waveform models while preserving the `crate::app::*` facade and fixing the resulting rustdoc boundary links.
- I have completed cleanup item 25 by splitting `src/app/controller/playback/player.rs` into focused transport start, player lifecycle, playhead follow-up, and waveform UI synchronization modules while preserving the public playback controller facade.
- I have completed cleanup item 26 by splitting `vendor/radiant/src/gui/native_shell/style.rs` into focused palette, sizing, and tier-policy modules while preserving the existing `StyleTokens` and `SizingTokens` contract surface.
- I have completed cleanup item 27 by extracting shared analysis-admin CLI bootstrap helpers for command execution, help detection, and default library DB resolution, and by adding parser tests around each binary's unique flags.
- I have completed cleanup item 28 by aligning the user-facing similarity-map naming across the public analysis surface, the `sempal-umap` admin tool, map/status text, and related docs while retaining the legacy `umap` compatibility shims and persisted names where needed.
- I have completed cleanup item 29 by replacing the STFT hot-path argument suppression with typed frame-processor and scratch structures and by removing the hardcoded fallback MFCC width in favor of the active mel configuration.
- I have completed cleanup item 30 by pruning dead `radiant` layout-helper surface, moving test-only layout/frame builders behind `#[cfg(test)]`, and documenting the remaining wider layout-policy suppressions as intentional compatibility surface.
- Cleanup Phase 2 for the refreshed backlog is complete through item 15.
- I have completed cleanup item 1 by splitting `src/app_core/native_bridge/metrics.rs` into focused registry, snapshot, and reporting modules while preserving the existing trace-hook facade and profiling output contract in commit `0286445a`.
- I have completed cleanup item 2 by moving the staged top-level native-shell app-model projection pipeline into `src/app_core/native_shell/app_model.rs`, leaving `src/app_core/native_shell.rs` as a thinner facade around shared motion and overlay helpers in commit `517ec252`.
- I have completed cleanup item 3 by splitting the recording waveform loader into focused IO, retained-state, incremental-update, and test modules while preserving the existing request-id and incremental-append behavior in commit `0524d980`.
- I have completed cleanup item 4 by turning `src/waveform/mod.rs` into a thin facade and moving the waveform public model and disk-loading entrypoints into focused `model` and `loading` modules while preserving the existing public imports in commit `5fea7bc1`.
- I have completed cleanup item 5 by splitting drag-drop action handling into focused drop-resolution, payload-finish, and external-drag modules while preserving the existing controller-facing drag/drop API in commit `e29d8464`.
- I have completed cleanup item 6 by splitting the GUI benchmark harness into focused workspace seeding, scenario registry, and report assembly modules in commit `d702d343`, then aligning the benchmark helper imports and waveform selection action fields with the current app contract in commit `4cc2ad7e`.
- I have completed cleanup item 7 by splitting `src/audio/source.rs` into focused source-combinator modules and extracting shared sample-accounting helpers for finite duration/sample limits and fade progression in commit `b68f80e5`.
- I have completed cleanup item 8 by splitting `src/analysis/audio/normalize.rs` into a thin facade plus dedicated runtime dispatch, scalar math, and x86 SIMD backend modules in commit `19bccb0c`.
- I have completed cleanup item 9 by splitting the remaining controller test hubs into focused `browser_actions`, `folders_core`, and `waveform` behavior-module trees in commits `2fc03b2c`, `9c1c3dfe`, and `5e309661`.
- I have completed cleanup item 10 by splitting the analysis-jobs DB tests into focused `samples`, `jobs`, `cleanup`, and `artifacts` modules backed by a shared schema/row fixture helper in commit `29e76b31`.
- I have completed cleanup item 11 by splitting `vendor/radiant` waveform overlay rendering into focused scrollbar, selection, edit-fade, and playhead-trail modules in vendor commit `36b8cd37`, and I aligned the related overlay tests to the micro-precision view-window contract.
- I have completed cleanup item 12 by splitting `vendor/radiant` static frame building into focused browser, map, waveform, and chrome builder modules in vendor commit `4501442d`, while keeping the native-shell paint order and quick CI behavior stable.
- I have completed cleanup item 13 by splitting `vendor/radiant/src/gui/native_shell/state.rs` into focused `cache_types`, `hit_testing`, and `toolbar_helpers` modules while preserving the public `NativeShellState` surface and passing targeted `radiant` state tests plus `ci_quick.ps1`.
- I am keeping `AGENTS.md`, `MEMORY.md`, `docs/README.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` aligned so wake-up context stays consistent.
- I have completed cleanup item 14 by splitting the native Vello runtime support surface into focused `profiling`, `runtime_state`, `scene_cache`, `startup`, and `text_bpm` modules in vendor commit `109818a4`, with the main repo pointer update in commit `3bf3b6a8`.
- I have completed cleanup item 15 by extracting shared companion-app native window/bootstrap helpers into `src/companion_apps/native_ui.rs` and routing both installer and updater-helper through that shared policy in commit `b3a3ded9`.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Cleanup items 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, and 30 are complete; they landed in commits `16932de4`, `1fe099ae`, `0b0be54a`, `f752dec6`, `8d2c30e8`, `30d25841`, `08541a52`, `d538fd60`, `b5702240`, `07afb548`, `1a0a20eb`, `bb7216dd`, `bceaaeeb`, `319cefdd`, `002ce1b9`, `d18e19dc`, `d13b38fe`, `cc0edd90`, `336b2c65`, `db2e99d7`, `53f70d56`, `b6f3eb6f`, `d00bdd08`, `9e4b092c`, `71f2d9bf`, `a1e1195b`, `4a3e6660`, `c3005581`, `cf6c233a`, `934ad10b`, `cbe1c428`, `981b5337`, and vendor commit `9e8b3708`.

## Immediate Next Actions

1. Treat the refreshed cleanup lane as complete through item 15 unless a new follow-up audit is opened.
2. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized when the next lane opens.
3. Continue using the required local gates before each push in the current environment.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, and the completed execution backlog is recorded in `tmp/perf_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
- Active cleanup backlog: `tmp/cleanup_plan.md` (refreshed 2026-03-11; Phase 2 complete, items 1-15 complete).




