# Quality Scorecard

This document is a lightweight scorecard for key domains/layers in Sempal. The
goal is to make quality gaps explicit so agents and humans can prioritize the
next improvements without rediscovering context.

Last reviewed: 2026-02-18

## Scoring rubric (0–5)

- 0: Unknown / hazardous / no guardrails
- 1: Fragile; frequent footguns; no enforcement
- 2: Usable but inconsistent; partial coverage
- 3: Solid baseline; clear conventions; some enforcement
- 4: Strong; automated guardrails; good observability
- 5: Excellent; hard to regress; fast to change safely

## Current scores

| Area | Score | Notes |
| --- | ---: | --- |
| Developer entrypoints (docs/scripts) | 4 | `docs/README.md`, `scripts/ci_local.*`, `scripts/doctor.*`, `scripts/run_sandbox.*` exist and are wired into CI/local flows. |
| Documentation hygiene | 4 | Knowledge lint exists; still some doc drift risk outside the checked scope. |
| Legacy boundary enforcement | 4 | `crate::app` coupling and `app_core` boundaries are enforced diff-aware in CI. |
| Code size discipline | 3 | File size budget enforced on changed files; allowlist is still large and needs burn-down. |
| Testing posture | 3 | Test map exists; coverage varies; some critical flows are integration-heavy. |
| Observability & diagnostics | 3 | Structured logging via `tracing` exists; log bundling helpers added; could improve targeted debug tooling. |
| Performance guardrails | 2 | Perf QA docs exist; enforcement is mostly manual; large-file hot paths remain. |
| Security posture (local app) | 3 | Some explicit safety checks (updater path validation, SQLite extension gating); still relies on careful review. |

## Known gaps (actionable)

- Reduce file size allowlist debt: prioritize splitting the top 5 largest files and deleting allowlist entries as they fall below 400 LOC.
- Add a “safe sandbox run” to the default dev workflow documentation so nobody accidentally runs against real data by default.
- Add a scheduled doc review cadence: review this file monthly and update scores based on current reality.
- Add one performance regression harness for a representative large dataset/view and run it in CI (even if it is a coarse threshold test).

## Notes

- Scores are intentionally coarse and should be updated when the system changes.
- When making large changes, update `docs/QUALITY_SCORE.md` in the same PR if it affects any score materially.

