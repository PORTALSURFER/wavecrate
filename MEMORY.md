# Agent Memory

Last Updated: 2026-03-04T15:04:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I have completed a new cleanup audit pass and rewritten `tmp/cleanup_plan.md` with a strict ROI-ranked backlog (Phase 1 complete).
- I am waiting for explicit user confirmation before starting Phase 2 implementation from `tmp/cleanup_plan.md`.
- I am using `docs/plans/active/cleanup_architecture_note.md` as boundary guidance for this pass.
- Audit evidence has been refreshed in `tmp/cleanup_audit_hotspots.md`.

## Immediate Next Actions

1. Paste the exact ordered backlog from `tmp/cleanup_plan.md` into chat and ask for explicit Phase 2 confirmation.
2. If confirmed, execute cleanup items one-by-one in strict order with CI, plan updates, commits, and push per item.
3. Keep `AGENTS.md`, `MEMORY.md`, and `tmp/cleanup_plan.md` synchronized after each milestone.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`.
- Runtime performance backlog: `tmp/perf_plan.md`.
- Runtime redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`.
- Short queue reference: `docs/plans/active/todo.md`.
