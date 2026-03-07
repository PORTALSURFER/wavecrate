# Agent Memory

Last Updated: 2026-03-07T22:24:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am executing Phase 2 of the runtime performance plan in `tmp/perf_plan.md`.
- Perf items 1-4 are complete, including lazy browser lookup-map rebuilds in commit `458f2f50`.
- The next ordered implementation target is `tmp/perf_plan.md` item 5: reduce whole-dataset search work and stale-query CPU burn.

## Immediate Next Actions

1. Implement `tmp/perf_plan.md` item 5 next and continue strictly in order.
2. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` aligned on every perf milestone commit.
3. Use `scripts/ci_quick.*` during the tight loop and `scripts/ci_local.*` plus `scripts/run_perf_guard.sh` for broader perf validation.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, with remaining ROI now concentrated in search/filter work, retained row metadata churn, and waveform image upload caching.
- Short queue reference: `docs/plans/active/todo.md`.
