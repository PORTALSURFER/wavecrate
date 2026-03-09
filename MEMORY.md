# Agent Memory

Last Updated: 2026-03-09T19:35:20Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed the Phase 2 runtime performance execution backlog in `tmp/perf_plan.md`.
- Perf items 1-11 are complete, including hot native input layout borrowing in vendor commit `fd542453` and root perf attribution work in commit `f239b03b`.
- The active source of truth for any follow-up perf work remains `docs/plans/active/runtime_performance_exec_plan.md`.
- I have refreshed `tmp/cleanup_plan.md` against the current `next` head with a 17-item strict ROI-ranked cleanup backlog.
- Cleanup Phase 2 is in progress from `tmp/cleanup_plan.md`.
- Cleanup items 1, 2, 3, 4, 5, 6, 7, and 8 are complete; they landed in commits `16932de4`, `1fe099ae`, `0b0be54a`, `f752dec6`, `8d2c30e8`, `30d25841`, `08541a52`, `d538fd60`, and `b5702240`.

## Immediate Next Actions

1. Use `docs/plans/active/runtime_performance_exec_plan.md` for any follow-up runtime/perf work beyond the completed `tmp/perf_plan.md` backlog.
2. Use `tmp/cleanup_plan.md` as the ordered cleanup backlog and implement items strictly in that order, starting from item 9.
3. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` synchronized when future work changes the current state.
4. Continue using `bash scripts/ci_quick.sh` and `bash scripts/ci_local.sh` as the required gates before each push.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, and the completed execution backlog is recorded in `tmp/perf_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
- Active cleanup backlog: `tmp/cleanup_plan.md`.
