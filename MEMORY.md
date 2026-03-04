# Agent Memory

Last Updated: 2026-03-04T16:52:41Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed a new cleanup Phase 1 audit and rewritten `tmp/cleanup_plan.md` as a fresh ROI-ranked backlog.
- `tmp/cleanup_plan.md` currently has 12 pending cleanup items (`[ ]`) in strict ROI order.
- Cleanup Phase 2 has not started; I am waiting for explicit user confirmation before implementation.
- Runtime/performance work remains tracked in `tmp/perf_plan.md`, but cleanup execution is currently the active front-door request.
- The latest baseline `bash scripts/ci_local.sh` run is green (with a perf warning for `wheel_latency` only).

## Immediate Next Actions

1. Present the exact ordered ROI backlog from `tmp/cleanup_plan.md` to the user.
2. If explicitly confirmed, execute cleanup items sequentially with CI + commit/push per item.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` synchronized at milestones.

## Work Notes

- Active cleanup backlog (pending): `tmp/cleanup_plan.md`.
- Runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
