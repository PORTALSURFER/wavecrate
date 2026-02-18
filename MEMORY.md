# Agent Memory

Last Updated: 2026-02-18T12:06:16Z
Updated By: Codex

## Purpose

- Maintain a lightweight, machine-checkable session memory for agent handoff.
- Record the latest work context and intent so the next agent can pick up immediately.

## Current Session (2026-02-18 UTC)

- Added `MEMORY.md` with an explicit UTC timestamp and updater marker.
- Added local CI enforcement so `scripts/ci_local.sh` (and `.ps1` equivalent) fails if this file is stale or missing an updater marker.
- Next agent action expectation: keep this file updated with a fresh UTC timestamp before/after substantial work.

## Work Notes

- Primary objective: make agent handoff and local CI behavior more explicit and self-healing.
- Next fix candidates from earlier review:
  - Repair `scripts/check_rust_taste_invariants.sh` syntax.
  - Fix regex matching in `scripts/check_file_size_budget.sh` so `--all` and path filters work.
  - Replace `.sempal_samples.db` default write path behavior in sandbox mode.
