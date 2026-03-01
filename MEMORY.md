# Agent Memory

Last Updated: 2026-03-01T21:50:02Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed item 1 of `tmp/perf_plan.md` and switched browser
  focus/selection hot paths to index-first execution to avoid repeated path
  re-resolution.
- I am executing Phase 2 sequentially in strict ROI order, one item at a time.
- Item 2 (projection cache key partitioning for cursor/playhead dynamics) is
  next.
- Preflight is green (`bash scripts/run_agent_request.sh`), and the latest perf
  evidence is from `target/perf/bench.json` generated during this pass.

## Immediate Next Actions

1. Execute item 2 in `tmp/perf_plan.md` (static vs motion cache-key split).
2. Continue remaining plan items in order; after each item run CI, commit, push,
   and mark completion with date/hash.
3. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` synchronized.

## Work Notes

- Detailed execution and rationale live in:
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Current performance execution plan lives in:
  `tmp/perf_plan.md`.
- Short ordered queue lives in:
  `docs/plans/active/todo.md`.
- Latest benchmark artifact:
  - `target/perf/bench.json`
