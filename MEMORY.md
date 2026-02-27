# Agent Memory

Last Updated: 2026-02-27T20:03:16Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am running a deep cleanup audit pass and have generated a strict ROI-ranked
  backlog at `tmp/cleanup_plan.md`.
- I am in Phase 1 (planning only): no cleanup implementation has started yet.
- I am waiting for explicit user confirmation before executing Phase 2
  sequentially in plan order.
- Local CI is green before planning edits (`bash scripts/ci_local.sh`).

## Immediate Next Actions

1. Wait for explicit user confirmation to begin Phase 2 implementation.
2. Execute `tmp/cleanup_plan.md` items strictly in order; after each item:
   run CI, commit, push, and mark the item complete with date/hash.
3. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` synchronized.

## Work Notes

- Detailed execution and rationale live in:
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Current cleanup execution plan lives in:
  `tmp/cleanup_plan.md`.
- Short ordered queue lives in:
  `docs/plans/active/todo.md`.
- Latest calibration artifacts:
  - `target/perf/bench.startup_calibration2.startup_summary.json`
  - `target/perf/startup_thresholds.lock.env`
  - `target/perf/wave_preview_on_calib.json`
  - `target/perf/wave_preview_off_calib.json`
