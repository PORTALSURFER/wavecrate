# Agent Memory

Last Updated: 2026-03-07T17:05:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed a fresh runtime performance audit and rewritten the ordered backlog in `tmp/perf_plan.md`.
- The current measured hotspots are bridge pull/projection invalidation and browser row-window projection, not waveform interaction or ordinary search dispatch.
- Phase 1 is active until the user explicitly approves Phase 2 implementation.

## Immediate Next Actions

1. If approved, implement `tmp/perf_plan.md` item 1 first and continue strictly in order.
2. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/perf_plan.md` aligned on every perf milestone commit.
3. Use `scripts/ci_quick.*` during the tight loop and `scripts/ci_local.*` plus `scripts/run_perf_guard.sh` for broader perf validation.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence came from `target/perf/bench.json`, with `interactive_projection` pull-stage p95 still dominating the remaining budget.
- Short queue reference: `docs/plans/active/todo.md`.
