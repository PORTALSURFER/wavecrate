# Agent Memory

Last Updated: 2026-03-01T09:18:01Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am running a deep runtime performance audit pass and have generated a strict
  ROI-ranked backlog at `tmp/perf_plan.md`.
- I am in Phase 1 (planning only): no performance implementation has started
  yet.
- I am waiting for explicit user confirmation before executing Phase 2
  sequentially in plan order.
- Preflight is green (`bash scripts/run_agent_request.sh`), and the latest perf
  evidence is from `target/perf/bench.json`.

## Immediate Next Actions

1. Wait for explicit user confirmation to begin Phase 2 implementation.
2. Execute `tmp/perf_plan.md` items strictly in order; after each item:
   run CI, commit, push, and mark the item complete with date/hash.
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
