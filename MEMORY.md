# Agent Memory

Last Updated: 2026-02-19T16:47:16Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and the current state of the agent-facing safety work.

## Current Session (2026-02-18 UTC)

- Working on P0/P1 hardening for safer agent-facing execution.
- Current branch changes focus on:
  - Added script-guard self-check fixtures for `scripts/check_file_size_budget.sh` and `scripts/check_rust_taste_invariants.sh`.
  - Added explicit parse-argument handling for check scripts and documented failures on malformed flags.
  - Added machine-readable run contract NDJSON artifacts in `src/main.rs`.
  - Documented run-contract schema for harness assertions in `docs/run_contracts.md`.
  - Hardened `run_sandbox`/`run_sandbox.ps1` messaging and behavior around read-only DB defaults and user-library overrides.
  - Added active/completed plan folders with index under `docs/plans/` and updated agent handoff docs.
  - Added source DB user-library guard messaging and retained read-only-first defaults.
  - Updated CI doc-index requirements and memory freshness flow to enforce session updates.
- Immediate next action: keep local CI and guardrail scripts green after every incremental change.

## Work Notes

- Fixed broken script guardrail tests by making fixtures run from a repo-shaped temporary directory (script lives under `scripts/`).
- Added deterministic user-root override helper in `src/sample_sources/db/mod.rs` tests so env-state races are guarded by a lock.
- Updated `scripts/check_rust_taste_invariants.sh` to avoid fragile regex escapes and reduce false parse failures.
