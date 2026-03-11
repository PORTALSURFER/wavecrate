# Agent Memory

Last Updated: 2026-03-11T07:47:22Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed the Phase 2 runtime performance execution backlog in `tmp/perf_plan.md`.
- Perf items 1-11 are complete, including hot native input layout borrowing in vendor commit `fd542453` and root perf attribution work in commit `f239b03b`.
- The active source of truth for any follow-up perf work remains `docs/plans/active/runtime_performance_exec_plan.md`.
- I have refreshed `tmp/cleanup_plan.md` against the current `next` head with a 30-item strict ROI-ranked cleanup backlog.
- I have completed cleanup item 14 by splitting `src/app/controller/playback/waveform_actions.rs` into focused waveform-action submodules while keeping the public controller surface stable.
- I have completed cleanup item 15 by splitting `src/app_core/native_bridge.rs` into focused batching, reducer, and projection-runtime modules and by splitting the native-bridge test surface into queue, bridge-runtime, and projection-cache modules.
- I have completed cleanup item 16 by splitting `src/sample_sources/db/schema.rs` into focused schema DDL and migration modules and by adding legacy migration fixtures for `wav_files`, `analysis_jobs`, and `samples`.
- I have completed cleanup item 17 by routing one-shot source DB mutations through `SourceWriteBatch`, consolidating wav upsert/update helpers, and adding wrapper-vs-batch parity coverage.
- I have completed cleanup item 18 by moving controller performance, status, undo, and small job/playback helpers out of `src/app/controller.rs`, leaving the root controller focused on construction and top-level accessors.
- Cleanup Phase 2 is in progress, and item 19 is the next pending cleanup step.
- I am keeping `AGENTS.md`, `MEMORY.md`, `docs/README.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` aligned so wake-up context stays consistent.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Cleanup items 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, and 18 are complete; they landed in commits `16932de4`, `1fe099ae`, `0b0be54a`, `f752dec6`, `8d2c30e8`, `30d25841`, `08541a52`, `d538fd60`, `b5702240`, `07afb548`, `1a0a20eb`, `bb7216dd`, `bceaaeeb`, `319cefdd`, `002ce1b9`, `d18e19dc`, `d13b38fe`, `cc0edd90`, and `336b2c65`.

## Immediate Next Actions

1. Continue cleanup item 19 from `tmp/cleanup_plan.md`.
2. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized when the active state changes.
3. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a new perf follow-up lane is opened after cleanup.
4. Continue using the required local gates before each push in the current environment.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, and the completed execution backlog is recorded in `tmp/perf_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
- Active cleanup backlog: `tmp/cleanup_plan.md`.


