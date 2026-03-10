# Agent Memory

Last Updated: 2026-03-10T15:19:58Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed the Phase 2 runtime performance execution backlog in `tmp/perf_plan.md`.
- Perf items 1-11 are complete, including hot native input layout borrowing in vendor commit `fd542453` and root perf attribution work in commit `f239b03b`.
- The active source of truth for any follow-up perf work remains `docs/plans/active/runtime_performance_exec_plan.md`.
- I have refreshed `tmp/cleanup_plan.md` against the current `next` head with a 30-item strict ROI-ranked cleanup backlog.
- I have revalidated the remaining cleanup hotspots against the current `next` head after the latest waveform and native-shell fixes, and item 14 is still the correct next step.
- Cleanup Phase 1 is complete, and Phase 2 implementation is still paused because it is waiting for explicit user confirmation.
- I have aligned `AGENTS.md`, `MEMORY.md`, `docs/README.md`, `docs/plans/index.md`, and `docs/plans/active/todo.md` so the active mission and next actions are obvious on wake-up.
- I have repaired the Bash-side diff-aware guardrails for this Windows-mounted worktree so `run_agent_request.sh` and `ci_local.sh` see the same real changes as the Windows git workflow.
- The required housekeeping validation passes: `bash scripts/run_agent_request.sh` and `bash scripts/ci_local.sh` are green in the current environment.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Cleanup items 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, and 13 are complete; they landed in commits `16932de4`, `1fe099ae`, `0b0be54a`, `f752dec6`, `8d2c30e8`, `30d25841`, `08541a52`, `d538fd60`, `b5702240`, `07afb548`, `1a0a20eb`, `bb7216dd`, `bceaaeeb`, and `319cefdd`.

## Immediate Next Actions

1. Use `tmp/cleanup_plan.md` as the ordered cleanup backlog and start item 14 only after the user explicitly confirms Phase 2.
2. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized when the active state changes.
3. Use `docs/plans/active/runtime_performance_exec_plan.md` only if a new perf follow-up lane is opened after cleanup.
4. Continue using the required local gates before each push in the current environment.

## Work Notes

- Active runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Bench evidence is still anchored in `target/perf/bench.json`, and the completed execution backlog is recorded in `tmp/perf_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
- Active cleanup backlog: `tmp/cleanup_plan.md`.
