# Agent Memory

Last Updated: 2026-03-09T11:27:30Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed the Phase 2 runtime performance execution backlog in `tmp/perf_plan.md`.
- Perf items 1-11 are complete, including hot native input layout borrowing in vendor commit `fd542453` and root perf attribution work in commit `f239b03b`.
- The active source of truth for any follow-up perf work remains `docs/plans/active/runtime_performance_exec_plan.md`.
- I have refreshed `tmp/cleanup_plan.md` with a new ROI-ranked cleanup backlog from the current codebase state.
- Cleanup item 5 from `tmp/cleanup_plan.md` is complete in commit `cbc3c480`, and item 6 is next.

## Immediate Next Actions

1. Start cleanup item 6 from `tmp/cleanup_plan.md` and refactor the source-move worker pipeline around explicit transactional stages.
2. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` synchronized as each cleanup item lands.
3. Continue using `scripts/ci_quick.ps1` and `scripts/ci_local.ps1` as the required gates before each push.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, and the completed execution backlog is recorded in `tmp/perf_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
- Active cleanup backlog: `tmp/cleanup_plan.md`.
