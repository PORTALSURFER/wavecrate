# Agent Memory

Last Updated: 2026-03-11T16:31:21Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `C:\dev\sempal`.
- I have completed the runtime performance backlog in `tmp/perf_plan.md` through item 11.
- I have completed the previous cleanup lanes recorded in older `tmp/cleanup_plan.md` revisions.
- I have now opened a fresh cleanup audit pass and rebuilt `tmp/cleanup_plan.md`.
- The new cleanup plan contains 16 pending items in strict ROI order.
- Phase 1 is complete; no implementation work from the new cleanup backlog has started.
- Phase 2 is blocked on explicit user confirmation.
- The current cleanup source of truth is `tmp/cleanup_plan.md`.
- The perf source of truth remains `docs/plans/active/runtime_performance_exec_plan.md` and stays dormant unless a separate perf lane is reopened.
- Future Windows sessions must not run the Bash workflow scripts; they should use only the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Paste the exact ordered backlog from `tmp/cleanup_plan.md` into chat.
2. Ask for explicit confirmation before starting cleanup item 1.
3. If the user confirms, execute the plan strictly in order and keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized after each milestone.

## Work Notes

- Active cleanup backlog: `tmp/cleanup_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Perf execution record: `tmp/perf_plan.md`
- Perf redesign source of truth: `docs/plans/active/runtime_performance_exec_plan.md`
