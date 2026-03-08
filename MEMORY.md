# Agent Memory

Last Updated: 2026-03-08T03:55:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am executing Phase 2 of the runtime performance plan in `tmp/perf_plan.md`.
- Perf items 1-10 are complete, including measured segment-probe timing attribution in commit `f239b03b`.
- The next ordered implementation target is `tmp/perf_plan.md` item 11: borrow `ShellLayout` through hot native input handlers instead of cloning it repeatedly.

## Immediate Next Actions

1. Implement `tmp/perf_plan.md` item 11 next and continue strictly in order.
2. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` aligned on every perf milestone commit.
3. Use `scripts/ci_quick.*` during the tight loop and `scripts/ci_local.*` plus `scripts/run_perf_guard.sh` for broader perf validation.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, with remaining ROI now concentrated in the final hot native input/layout ownership cleanup item.
- Short queue reference: `docs/plans/active/todo.md`.
