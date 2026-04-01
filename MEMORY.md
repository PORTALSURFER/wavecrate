# Agent Memory

Last Updated: 2026-03-31T19:55:00Z
Updated By: Codex

## Purpose

- Keep session handoff durable for stateless agent sessions.
- Record what is happening now and what happens next.

## Current State (Present Tense)

- I am on `next` in `X:\sempal`.
- `X:\sempal` and `X:\sempal\vendor\radiant` are still expected to stay on local `next`.
- I am executing Phase 2 of the refreshed evidence-driven improvement audit for the current live tree.
- `tmp/improvement_audit_plan.md` is the current source of truth and execution record for this lane.
- Item 1 is complete: the cleanup-hotspot audit heuristic now recognizes `*_tests.rs` and sibling module coverage declared through `mod.rs` plus `tests.rs`.
- Item 2 is complete: `docs/README.md` now points to `tmp/improvement_audit_plan.md` as the canonical audit-lane status source, and the docs-index/guardrail scripts enforce that phase-neutral pointer contract.
- The truthful full-scan budget is currently red and reports `20` non-allowlisted over-budget Rust files on the live tree.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`.
- A prerequisite baseline fix is also complete: the transient benchmark-source pruning coverage now lives beside the pure config helper, which removed the controller-level test interaction that had been crashing `cargo test -p sempal --lib -- --test-threads=1`.
- `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` is green again on the live tree.
- The refreshed backlog now continues with item 3 (`app_core` native dispatch-hub splits), then keyboard-path deduplication in `vendor/radiant`, the remaining production file-size debt, oversized test hubs, and `QUALITY_SCORE` drift hardening.
- The live audit currently records three open questions: playback-age/mark behavior as product contract, the canonical source for mutable lane-state docs, and stable GUI action-id ownership across `radiant` and `app_core`.
- `tmp/cleanup_plan.md` remains parked and should stay dormant unless the user explicitly reopens cleanup work.
- `tmp/perf_plan.md` remains parked and should stay dormant unless the user explicitly reopens performance work.
- Future Windows sessions must use the PowerShell wrappers in `scripts/*.ps1` unless the user explicitly overrides that rule.
- Rust tests must run serially in one cargo process at a time; do not run multiple Rust test processes concurrently.

## Immediate Next Actions

1. Execute item 3 from `tmp/improvement_audit_plan.md`: split the migration-facing native action dispatch hubs in `app_core` into smaller surface-specific helpers with direct local tests.
2. Keep `AGENTS.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` aligned with the active Phase 2 state as items advance.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable.
5. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.

## Work Notes

- Active audit plan: `tmp/improvement_audit_plan.md` (rebuilt on 2026-03-31; Phase 2 active, items 1 and 2 complete, item 3 next)
- Current hotspot snapshot: `tmp/cleanup_audit_hotspots.md`
- Active short queue: `docs/plans/active/todo.md`
- Dual-lane validation reference: `docs/TEST.md`
- Parked cleanup backlog: `tmp/cleanup_plan.md`
- Parked perf backlog: `tmp/perf_plan.md`
- GUI automation/test design: `docs/gui_test_platform.md`
- GUI automation/test rollout plan: `docs/plans/active/gui_test_platform_exec_plan.md`

