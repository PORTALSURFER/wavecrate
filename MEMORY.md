# Agent Memory

Last Updated: 2026-03-04T11:43:44Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed a fresh Phase 1 cleanup audit and written a strict ROI-ranked backlog to `tmp/cleanup_plan.md`.
- I am waiting for explicit user confirmation before starting Phase 2 sequential implementation.
- Runtime performance work from `tmp/perf_plan.md` remains active but is paused behind this cleanup request; item 7 is still the next perf item when resumed.
- Pre-edit CI is green (`bash scripts/ci_local.sh`).

## Immediate Next Actions

1. Present the exact ordered ROI backlog from `tmp/cleanup_plan.md` to the user and request explicit Phase 2 confirmation.
2. If confirmed, execute backlog items in strict order: implement, validate, mark done with date/hash, commit, push.
3. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` synchronized after each milestone.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`.
- Runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
