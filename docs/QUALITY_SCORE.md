# Quality Scorecard

This document is a lightweight scorecard for key domains/layers in Sempal. The
goal is to make quality gaps explicit so agents and humans can prioritize the
next improvements without rediscovering context.

Last reviewed: 2026-04-02

Scope note: this review describes the live observed workspace on 2026-04-02,
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
| Agent-facing guardrails | 4 | `scripts/check_file_size_budget.*` and `scripts/check_rust_taste_invariants.*` are both green again, and `scripts/check_quality_score_drift.*` now enforces that this score stays aligned with the live guardrail state. |
| Legacy boundary enforcement | 4 | `crate::app` coupling and `app_core` boundaries are enforced diff-aware in CI; the current browser playback-age drift was repaired back onto the intended `app_core` boundary. |
| Code size discipline | 3 | The repository has an explicit 400-line policy and allowlist, and the live full-scan file-size budget is currently green. Ongoing hotspot review still matters because several intentional exceptions and large test clusters remain easy to regress. |
| Testing posture | 3 | Focused unit coverage improved in transport/browser actions, but some critical flows remain integration-heavy. |
| Observability & diagnostics | 3 | Structured logging via `tracing` exists; log bundling helpers added; could improve targeted debug tooling. |
| Performance guardrails | 3 | `scripts/run_perf_guard.sh` is part of local CI; warning drift (for example `wheel_latency`) still needs ongoing burn-down. |
| Security posture (local app) | 3 | Some explicit safety checks (updater path validation, SQLite extension gating); still relies on careful review. |

## Known gaps (actionable)

- Keep the full-scan file-size budget green and use `tmp/cleanup_audit_hotspots.md` as the live hotspot snapshot when new large-file debt appears.
- Continue burning down the suppression debt surfaced in the refreshed hotspot snapshot instead of restating brittle exact counts here.
- Prefer linking to live guardrail sources (`scripts/check_*`, `tmp/improvement_audit_plan.md`, `tmp/cleanup_audit_hotspots.md`) when exact blocker lists or counts are likely to drift quickly.
- Add a scheduled doc review cadence: review this file monthly and update scores based on current reality.
- Add one performance regression harness for a representative large dataset/view and run it in CI (even if it is a coarse threshold test).
- The guardrail drift checks now live in `scripts/check_quality_score_drift.sh` and `scripts/check_quality_score_drift.ps1`, and are wired into local CI and GitHub CI.

## Notes

- Scores are intentionally coarse and should be updated when the system changes.
- When making large changes, update `docs/QUALITY_SCORE.md` in the same PR if it affects any score materially.
