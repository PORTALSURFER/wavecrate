# Agent Memory

Last Updated: 2026-04-18T22:41:57Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- The current workspace is dirty with unrelated user edits; I must not overwrite them while executing the perf lane.
- The user explicitly reopened the improvement-audit lane on 2026-04-19.
- `tmp/improvement_audit_plan.md` is now refreshed as the active Phase 1 evidence-driven audit backlog for the current live tree.
- `docs/plans/index.md` and `docs/plans/active/todo.md` now exist as the stable docs-side plan navigation layer.
- Runtime-performance follow-up history is no longer anchored by a live `tmp/perf_plan.md` file in this checkout; do not point wake-up docs or guardrails at that removed path.
- The current ordered improvement backlog is:
  - `OPT-52` restore script entrypoint and guardrail path consistency after the `scripts/internal` migration
  - `OPT-50` reuse the existing controller job-DTO split
  - `OPT-54` split file-op apply flows from folder mutation execution/recovery paths
  - `OPT-53` split browser visible-row pipeline stages into focused modules
- The Windows Cargo wrapper lane is still trustworthy in this environment because `scripts/internal/use_cargo_cache.ps1` falls back to a local passthrough `rustc` wrapper when user-level Cargo config forces a broken `sccache`.
- `tmp/improvement_audit_plan.md` remains the active improvement lane source of truth unless the user redirects to another issue or plan.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Treat `tmp/improvement_audit_plan.md` as the active audit source of truth until the user confirms a different lane.
2. Use `docs/plans/index.md` and `docs/plans/active/todo.md` as the durable plan-navigation layer.
3. Preserve the Windows PowerShell wrapper path for future validation runs in this environment.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Database-system audit notes: `tmp/database_system_audit_plan.md`
- Source-runtime test-isolation audit notes: `tmp/source_runtime_test_isolation_audit_plan.md`
- GUI automation/test design: `docs/SYSTEMS.md`





