# Cleanup Plan (ROI Ranked)

Generated: 2026-03-04 (UTC)
Phase: 1 audit complete; Phase 2 pending explicit user confirmation
Status legend: `[ ]` pending, `[x]` done

## Ordered Backlog

- [x] 1) Decompose `poll_background_jobs` into focused message handlers and add dispatch tests
  - ROI/Effort: High / M
  - Why it matters: One function currently owns cancellation handling, queue draining, stale-request guards, progress updates, and all job result routing. This raises regression risk and makes behavioral review difficult.
  - Evidence:
    - `src/app/controller/library/background_jobs/mod.rs` is 502 LOC.
    - `poll_background_jobs` spans 485 lines at `src/app/controller/library/background_jobs/mod.rs:18`.
    - No local tests/`#[cfg(test)]` in `src/app/controller/library/background_jobs/mod.rs`.
  - Recommended change: Split cancellation pre-pass + per-message handlers into dedicated helper modules (for example `handlers/audio.rs`, `handlers/file_ops.rs`, `handlers/updates.rs`) and add focused tests for stale message rejection and progress-state transitions.
  - Risk/tradeoffs: Medium. Reordering message side effects can cause subtle behavior drift if invariants are not preserved.
  - Suggested validation: targeted background-job handler tests, `cargo test -p sempal background_jobs`, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `c776f452`

- [x] 2) Split search worker pipeline (`process_search_job`) into staged functions with cancellation guards per stage
  - ROI/Effort: High / M
  - Why it matters: Search filtering/scoring is latency-sensitive, but one long function mixes DB reopen/reload, cache invalidation, scoring, similar sorting, and output shaping.
  - Evidence:
    - `process_search_job` spans 343 lines at `src/app/controller/library/wavs/browser_search_worker/pipeline.rs:18`.
    - Cancellation checks are repeated across many branches (`search_job_canceled*` calls across `pipeline.rs:25-370`).
    - No local tests in `src/app/controller/library/wavs/browser_search_worker/pipeline.rs`.
  - Recommended change: Extract stage helpers (`load_or_refresh_entries`, `compute_scores`, `build_visible_rows`, `finalize_result`) and add tests for cancellation cutoff, query-cache reuse, folder-filter hash invalidation, and sort-mode correctness.
  - Risk/tradeoffs: Medium. Search ranking/sort order can regress if stage contracts are not explicit.
  - Suggested validation: search-worker unit tests under `src/app/controller/library/wavs/browser_search_worker`, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `e90400b8`

- [x] 3) Remove duplicated rollback/journal error branches across source/folder move workers
  - ROI/Effort: High / M
  - Why it matters: The staged-move workers still duplicate long rollback/journal cleanup sequences, which makes file-op fixes expensive and inconsistent.
  - Evidence:
    - `run_source_move_task` at `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:297`.
    - `run_folder_sample_move_task` at `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:62`.
    - Repeated `rollback_staged_move_to_source`/`remove_move_journal_entry` blocks in both files (for example source worker `:410-480`, folder worker `:156-240`).
  - Recommended change: Introduce a shared failure-handling helper that encapsulates rollback + journal cleanup + progress reporting, and reuse it in both workers.
  - Risk/tradeoffs: Medium. Shared helper bugs affect both move paths.
  - Suggested validation: drag-drop move/rollback tests (including cancellation and DB-write failures), then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `0d6b20cf`

- [x] 4) Continue splitting `ControllerJobs` into domain modules (`dto`, `state`, `lifecycle`, `dispatch`)
  - ROI/Effort: High / L
  - Why it matters: `ControllerJobs` still combines message DTO definitions, worker handles, lifecycle/shutdown behavior, queue dispatch, and request-id/state management in one file.
  - Evidence:
    - `src/app/controller/jobs.rs` is 1237 LOC.
    - `ControllerJobs` starts at `src/app/controller/jobs.rs:573` and mixes many responsibilities through the remainder of the file.
  - Recommended change: Move DTO/result types and job-state types into focused submodules, keeping `jobs.rs` as an orchestration facade.
  - Risk/tradeoffs: Medium. Many call-sites depend on existing imports and visibility.
  - Suggested validation: controller job unit tests + integration smoke for scan/file-op/update flows, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `4cf0fa15`

- [x] 5) Break `playback/mod.rs` API facade into smaller responsibility files
  - ROI/Effort: High / L
  - Why it matters: Playback API entrypoints, waveform zoom/cursor actions, deferred persistence, tagging/navigation wrappers, and internal helpers all live in one large module.
  - Evidence:
    - `src/app/controller/playback/mod.rs` is 1031 LOC.
    - Dense wrapper surface across `mod.rs:98-643` and internal helper logic around `mod.rs:645-706`.
    - Adjacent `transport` module is also large (`src/app/controller/playback/transport.rs` at 708 LOC).
  - Recommended change: Split façade by behavior (`waveform_actions.rs`, `selection_actions.rs`, `persistence_flush.rs`, `navigation_actions.rs`) while preserving public controller methods.
  - Risk/tradeoffs: Medium-high. Public method wiring is broad and heavily used by UI/runtime paths.
  - Suggested validation: playback + waveform controller tests (including zoom anchor and deferred commit paths), then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `0844eaf2`

- [x] 6) Refactor native bridge metrics logging into snapshot + formatter layers
  - ROI/Effort: Medium / M
  - Why it matters: Profiling counters are useful, but metrics aggregation and logging are concentrated in one very large function with no direct unit tests.
  - Evidence:
    - `src/app_core/native_bridge/metrics.rs` is 791 LOC.
    - `maybe_log_bridge_profile` spans 284 lines at `src/app_core/native_bridge/metrics.rs:185`.
    - No local tests in `src/app_core/native_bridge/metrics.rs`.
  - Recommended change: Introduce a `BridgeMetricsSnapshot` builder and a formatter/helper module so logging output and counter math are independently testable.
  - Risk/tradeoffs: Medium-low. Feature-gated behavior must remain no-op in non-metrics builds.
  - Suggested validation: metrics feature tests (`--features native-bridge-metrics`) + `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `742b0387`

- [x] 7) Split `selection.rs` into focused range/state modules and externalize tests
  - ROI/Effort: Medium / M
  - Why it matters: Selection math, fade/gain modeling, drag state transitions, and a large inline test module are co-located, reducing navigability.
  - Evidence:
    - `src/selection.rs` is 889 LOC.
    - Core types are mixed in one file (`SelectionRange` near `selection.rs:49`, `SelectionState` near `selection.rs:408`, tests start at `selection.rs:600`).
  - Recommended change: Move pure range/fade math into `selection/range.rs`, drag/state machine logic into `selection/state.rs`, and tests into dedicated test modules.
  - Risk/tradeoffs: Medium-low. Public API surface in `selection` is broadly used and must remain stable.
  - Suggested validation: selection unit tests + controller waveform selection tests, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `b5e04073`

- [x] 8) Reduce `issue_gateway/token_store.rs` production-file bloat by moving inline tests
  - ROI/Effort: Medium / S
  - Why it matters: Security-sensitive production logic still shares a large file with extensive test fixtures/cases, which slows review and increases cognitive load.
  - Evidence:
    - `src/issue_gateway/token_store.rs` is 790 LOC.
    - Inline tests begin at `src/issue_gateway/token_store.rs:358` and run through the end of file.
  - Recommended change: Move token-store tests into `token_store/tests/*.rs` (or sibling `tests.rs`) and keep the main module focused on runtime code.
  - Risk/tradeoffs: Low. Primarily structural movement.
  - Suggested validation: `cargo test -p sempal issue_gateway::token_store`, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `f8335dcc`

- [x] 9) Remove or narrow non-test `dead_code` allowances in core app modules
  - ROI/Effort: Medium / S
  - Why it matters: Broad suppressions can hide stale APIs and weaken warning signal quality.
  - Evidence:
    - `src/lib.rs:8` has `#[allow(dead_code)] mod app;`.
    - `src/sample_sources/mod.rs` has `#[allow(dead_code)]` on methods/functions at lines `40`, `82`, `88`, `95`.
    - `src/app/controller/ui/hotkeys/types.rs` suppresses unused variants at lines `135-142`.
  - Recommended change: Verify usage; remove dead code where possible, otherwise gate with narrower cfg/test-only annotations and document rationale.
  - Risk/tradeoffs: Low-medium. Some APIs may be intentionally retained for binary/test wiring.
  - Suggested validation: `cargo clippy --all-targets`, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `050160c0`

- [x] 10) Replace crate-wide `too_many_arguments` suppressions with typed parameter objects in top hotspots
  - ROI/Effort: Medium / M
  - Why it matters: Broad file-level suppressions hide call-site complexity and make APIs harder to evolve safely.
  - Evidence:
    - File-level allowances in core paths such as:
      - `src/waveform/render.rs:1`
      - `src/waveform/render/cache.rs:1`
      - `src/sample_sources/db/file_ops_journal.rs:1`
      - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:1`
      - `src/app/controller/library/wavs/waveform_loading.rs:1`
  - Recommended change: Start with 2-3 highest-churn call paths; introduce small config/input structs to collapse related arguments and remove local suppressions incrementally.
  - Risk/tradeoffs: Medium. Signature changes can create broad call-site churn.
  - Suggested validation: targeted unit tests for refactored call chains + `cargo clippy --all-targets` + `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `7cc879ac`

- [x] 11) Close crate-visible documentation gaps in high-churn controller/runtime helpers
  - ROI/Effort: Low / M
  - Why it matters: Many `pub(crate)` APIs are used across modules but lack intent/constraint docs, increasing onboarding and maintenance cost.
  - Evidence:
    - Missing docs on crate-visible functions in high-churn paths (examples):
      - `src/app/controller/playback/mod.rs:65` (`bpm_min_selection_seconds`)
      - `src/app/controller/playback/mod.rs:82` (`selection_meets_bpm_min_for_playback`)
      - numerous crate-visible helpers reported by audit query across `src/app/controller/library/**` and `src/app/controller/playback/**`.
  - Recommended change: Add concise docs for what/why/constraints on crate-visible functions in top hotspots first (background jobs, playback, search pipeline, selection edits).
  - Risk/tradeoffs: Low. Documentation churn only.
  - Suggested validation: `RUSTDOCFLAGS='-D warnings' cargo doc -p sempal --no-deps` and `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `145a740d`

- [x] 12) Tighten cleanup-audit test-gap heuristic to exclude dedicated `tests.rs` files
  - ROI/Effort: Low / S
  - Why it matters: Current hotspot report shows known test files as “test-gap” candidates, adding noise to planning.
  - Evidence:
    - `tmp/cleanup_audit_hotspots.md` “Likely test-gap hotspots” includes `src/app_core/native_bridge/tests.rs` and `src/app_core/native_shell/tests.rs`.
    - Script currently filters `*/tests/*`, `tests/*`, and `*_test.rs`, but not `*/tests.rs`.
  - Recommended change: Update `scripts/audit_cleanup_hotspots.sh` filtering to skip `tests.rs` files and document the heuristic in script usage/help text.
  - Risk/tradeoffs: Low. Report-only behavior change.
  - Suggested validation: rerun script and confirm test-gap list no longer includes dedicated test modules, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `f41028ad`

## Progress Log

- 2026-03-04: Phase 1 refreshed from current code state; awaiting explicit user confirmation before Phase 2 implementation.
- 2026-03-04: Completed item 11 documentation pass for high-churn crate-visible controller/runtime helpers.
- 2026-03-04: Completed item 12 cleanup-audit heuristic update to exclude dedicated `tests.rs` modules from test-gap output.
