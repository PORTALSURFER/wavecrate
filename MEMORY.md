# Agent Memory

Last Updated: 2026-02-23T11:40:12Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am running a docs-only housekeeping pass to improve wake-up clarity and
  reduce handoff drift.
- The active mission remains runtime responsiveness/performance redesign from
  `docs/plans/active/runtime_performance_exec_plan.md`.
- The latest shipped milestone is ROI item #9: waveform projection now avoids
  deep image payload clones by using shared `Arc`-backed image buffers.
- Preflight (`bash scripts/run_agent_request.sh`) is green in this session,
  including full local CI (perf guard warnings are warn-only).

## Immediate Next Actions

1. Calibrate startup-profile thresholds on a compositor-backed host and lock
   environment defaults.
2. Re-run immediate waveform-preview A/B on a compositor-backed host with
   larger run windows, then decide whether to widen immediate apply scope.
3. Keep `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` synchronized
   on every milestone commit.

## Work Notes

- Detailed execution and rationale live in:
  `docs/plans/active/runtime_performance_exec_plan.md`.
- Short ordered queue lives in:
  `docs/plans/active/todo.md`.
