# Agent Memory

Last Updated: 2026-02-18T12:21:28Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and the current state of the agent-facing safety work.

## Current Session (2026-02-18 UTC)

- Working on P0 hardening for safer agent-facing behavior.
- Current branch changes focus on:
  - Added fixture-based script self-checks in `scripts/check_script_guardrails.sh`.
  - Hardened source DB opening with explicit read-only defaults and user-library write guards.
  - Updated sandbox runtime defaults to read-only DB behavior.
  - Added explicit user-library override handling for db writes.
  - Added session memory freshness enforcement in local CI checks.
- Immediate next action: keep local CI and guardrail scripts green after every incremental change.

## Work Notes

- Fixed broken script guardrail tests by making fixtures run from a repo-shaped temporary directory (script lives under `scripts/`).
- Added deterministic user-root override helper in `src/sample_sources/db/mod.rs` tests so env-state races are guarded by a lock.
- Updated `scripts/check_rust_taste_invariants.sh` to avoid fragile regex escapes and reduce false parse failures.
