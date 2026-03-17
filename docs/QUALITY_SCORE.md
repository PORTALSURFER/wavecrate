# Quality Scorecard

This document is a lightweight scorecard for key domains/layers in Sempal. The
goal is to make quality gaps explicit so agents and humans can prioritize the
next improvements without rediscovering context.

Last reviewed: 2026-03-17

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
| Agent-facing guardrails | 4 | Diff-aware docs, file-size, taste, and boundary guardrails are wired into local CI and currently passing on the active branch. |
| Legacy boundary enforcement | 4 | `crate::app` coupling and `app_core` boundaries are enforced diff-aware in CI. |
| Code size discipline | 3 | File size budget is enforced without live allowlist entries on the current tree, and `tmp/cleanup_audit_hotspots.md` tracks the broader hotspot set for remaining full-scan debt. |
| Testing posture | 3 | Focused unit coverage improved in transport/browser actions, but some critical flows remain integration-heavy. |
| Observability & diagnostics | 3 | Structured logging via `tracing` exists; log bundling helpers added; could improve targeted debug tooling. |
| Performance guardrails | 3 | `scripts/run_perf_guard.sh` is part of local CI; warning drift (for example `wheel_latency`) still needs ongoing burn-down. |
| Security posture (local app) | 3 | Some explicit safety checks (updater path validation, SQLite extension gating); still relies on careful review. |

## Known gaps (actionable)

- Reduce the remaining full-scan file-size debt tracked in `tmp/cleanup_audit_hotspots.md`: prioritize `src/app/controller/tests/drag_drop_drop_targets.rs`, `src/analysis/ann_index_tests.rs`, `src/sample_sources/scanner/scan/tests.rs`, `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs`, `src/app/controller/tests/browser_selection.rs`, `src/app/controller/playback/tests.rs`, and `src/selection/range.rs`.
- Continue burning down the remaining `#[allow(dead_code)]` suppression in `src/lib.rs`; `clippy::too_many_arguments` suppressions are currently at zero on the active tree.
- Add a scheduled doc review cadence: review this file monthly and update scores based on current reality.
- Add one performance regression harness for a representative large dataset/view and run it in CI (even if it is a coarse threshold test).
- The guardrail drift check now lives in `scripts/check_quality_score_drift.sh` and is wired into local CI and GitHub CI.

## Notes

- Scores are intentionally coarse and should be updated when the system changes.
- When making large changes, update `docs/QUALITY_SCORE.md` in the same PR if it affects any score materially.
