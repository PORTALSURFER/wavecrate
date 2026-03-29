# Quality Scorecard

This document is a lightweight scorecard for key domains/layers in Sempal. The
goal is to make quality gaps explicit so agents and humans can prioritize the
next improvements without rediscovering context.

Last reviewed: 2026-03-29

Scope note: this review describes the live observed workspace on 2026-03-29,
which is currently dirty. It is not a claim that the last clean baseline has
the same guardrail posture.

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
| Agent-facing guardrails | 3 | Guardrails are still wired into local CI and the quality-score drift check now reports degraded state correctly on Windows, but the observed full file-size scan is red again on this live tree and should not be described as healthy. |
| Legacy boundary enforcement | 4 | `crate::app` coupling and `app_core` boundaries are enforced diff-aware in CI. |
| Code size discipline | 2 | The observed full-scan budget currently fails with 30 over-budget files across the root repo and `vendor/radiant`, and `tmp/cleanup_audit_hotspots.md` now surfaces a much larger over-budget/runtime hotspot cluster than the allowlist alone describes. |
| Testing posture | 3 | Focused unit coverage improved in transport/browser actions, but some critical flows remain integration-heavy. |
| Observability & diagnostics | 3 | Structured logging via `tracing` exists; log bundling helpers added; could improve targeted debug tooling. |
| Performance guardrails | 3 | `scripts/run_perf_guard.sh` is part of local CI; warning drift (for example `wheel_latency`) still needs ongoing burn-down. |
| Security posture (local app) | 3 | Some explicit safety checks (updater path validation, SQLite extension gating); still relies on careful review. |

## Known gaps (actionable)

- Restore the enforced full-scan file-size budget to green on a clean baseline; the observed live tree currently reports 30 active-scope violations, while `tmp/cleanup_audit_hotspots.md` reports 58 over-budget Rust files across the broader snapshot.
- Continue burning down the suppression debt now surfaced in the refreshed hotspot snapshot; the observed tree currently has `#[allow(dead_code)]` in 2 files and `clippy::too_many_arguments` in 3 files.
- Add a scheduled doc review cadence: review this file monthly and update scores based on current reality.
- Add one performance regression harness for a representative large dataset/view and run it in CI (even if it is a coarse threshold test).
- The guardrail drift check now lives in `scripts/check_quality_score_drift.sh` and is wired into local CI and GitHub CI.

## Notes

- Scores are intentionally coarse and should be updated when the system changes.
- When making large changes, update `docs/QUALITY_SCORE.md` in the same PR if it affects any score materially.
