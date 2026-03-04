# Cleanup Plan (ROI Ranked)

Generated: 2026-03-04 (UTC)
Phase: 1 audit complete; Phase 2 pending explicit user confirmation
Status legend: `[ ]` pending, `[x]` done
Canonical local CI command: `bash scripts/ci_local.sh`

## Ordered Backlog

- [x] 1) Repair stale file-size allowlist entries and remove no-longer-needed exemptions
  - ROI/Effort: High / S
  - Why it matters: Stale allowlist rows hide real budget regressions and create maintenance noise in every file-size audit.
  - Evidence:
    - `bash scripts/report_file_size_budget_allowlist.sh` reports `missing=2` and `ok=11` entries that can be removed.
    - Missing entries include `src/app/controller/playback/audio_loader.rs` and `src/selection.rs`.
    - `docs/file_size_budget_allowlist.txt` still contains now-under-budget entries like `src/app/controller/library/wavs.rs` (`:26`) and stale paths like `src/selection.rs` (`:65`).
  - Recommended change: Remove stale/missing allowlist rows, prune all now-under-budget rows, and keep only active over-budget exceptions.
  - Risk/tradeoffs: Low. Only guardrail metadata changes.
  - Suggested validation: `bash scripts/report_file_size_budget_allowlist.sh` should show `missing=0` and no removable rows; then run `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `8bb36ecc`

- [x] 2) Make folder-move DB-failure regression test deterministic (remove timing race)
  - ROI/Effort: High / S
  - Why it matters: A flaky core file-op test blocks CI confidence and wastes iteration time.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs:153` test `folder_sample_move_db_write_failure_rolls_back_source_and_keeps_journal_for_recovery` uses timing-based lock orchestration.
    - The test currently sleeps for 7 seconds (`:177`) and uses a 1-second receive timeout (`:180`), which is race-prone under load.
  - Recommended change: Replace sleep-based lock timing with deterministic synchronization (explicit lock-held/latch signaling and guaranteed release scope).
  - Risk/tradeoffs: Low-medium. Test harness changes must preserve intended DB-failure semantics.
  - Suggested validation: run targeted test repeatedly (looped `cargo test` on this test) and then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `fa24a4ff`

- [ ] 3) Split background-job polling into focused message-router and handler modules with behavior tests
  - ROI/Effort: High / L
  - Why it matters: Job polling is a high-change integration hotspot; current monolith shape increases coupling and regression risk.
  - Evidence:
    - `src/app/controller/library/background_jobs/polling.rs` is 676 LOC.
    - `handle_background_job_message` has a large `JobMessage` dispatch match at `:50-107` with many variant paths.
    - Tests only cover helper predicates/mapping (`:575+`) and not most handler side effects.
  - Recommended change: Extract routing + per-domain handlers (`audio`, `scan`, `file_ops`, `analysis`, `updates`) and add scenario tests for stale-message drops and progress/state transitions.
  - Risk/tradeoffs: Medium-high. Dispatch rewiring can break subtle state/progress behavior.
  - Suggested validation: targeted controller background-job tests plus `bash scripts/ci_local.sh`.

- [ ] 4) Decompose browser actions facade by responsibility and tighten focused tests
  - ROI/Effort: High / L
  - Why it matters: Browser interaction logic is central UX behavior; one broad file obscures invariants around focus/selection/commit flows.
  - Evidence:
    - `src/app/controller/library/wavs/browser_actions.rs` is 687 LOC.
    - File exposes 33 public/controller-facing functions (`rg` count) spanning focus, selection ranges, rename, delete, and explorer reveal.
    - Local tests in-file are limited to three scenarios (`:612`, `:635`, `:665`).
  - Recommended change: Split into modules (`focus_nav`, `selection_ranges`, `row_actions`) and add focused tests for anchor/selection invariants and commit-vs-preview transitions.
  - Risk/tradeoffs: Medium. Call-site reshaping can introduce behavior drift if invariants are not asserted.
  - Suggested validation: browser action unit/integration tests and `bash scripts/ci_local.sh`.

- [ ] 5) Separate folder-delete recovery journal logic, recovery executor, and UI projection; expand recovery matrix tests
  - ROI/Effort: High / L
  - Why it matters: Crash-recovery code is safety-critical and should be easy to reason about in failure scenarios.
  - Evidence:
    - `src/app/controller/library/source_folders/delete_recovery.rs` is 686 LOC.
    - It currently mixes filesystem staging, journal persistence, recovery policy, and UI/report application in one module.
    - Only one local test exists (`:678`), despite many failure paths.
    - Uses direct stderr output at `:308` instead of structured app logging.
  - Recommended change: Split into `journal`, `recovery`, and `controller_apply` modules; add tests for staged-intent/db-committed/unjournaled recovery permutations.
  - Risk/tradeoffs: Medium-high. Recovery sequencing is sensitive to ordering and rollback behavior.
  - Suggested validation: dedicated recovery matrix tests + existing folder tests + `bash scripts/ci_local.sh`.

- [ ] 6) Refactor source-move worker pipeline to isolate DB/fs stage transitions and error handling
  - ROI/Effort: High / M
  - Why it matters: Source move operations mutate FS + DB + journal; compact stage boundaries reduce recovery bugs.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` is 701 LOC.
    - `run_source_move_request` (`:389-482`) mixes destination resolution, metadata load, journal prep, DB updates, and finalize rename in one flow.
    - Error reporting still prints per-error lines via `eprintln!` (`:226`).
  - Recommended change: Introduce explicit per-stage helpers with typed stage outcomes and central error/report policy.
  - Risk/tradeoffs: Medium. Refactoring transactional flow can change rollback behavior if stage contracts are unclear.
  - Suggested validation: source-move success/failure/cancel tests and `bash scripts/ci_local.sh`.

- [ ] 7) Standardize controller/worker error logging (`eprintln!` -> structured `tracing`)
  - ROI/Effort: Medium / M
  - Why it matters: Mixed stderr logging is hard to filter in production and inconsistent with tracing-based diagnostics.
  - Evidence:
    - Controller/worker paths still use direct stderr in multiple files, e.g. `src/app/controller/ui/file_ops.rs:97`, `src/app/controller/library/trash.rs:189`, `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:226`, `src/app/controller/library/source_folders/delete_recovery.rs:308`, and analysis workers in `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs`.
  - Recommended change: Replace direct stderr prints with structured `tracing::{warn,error,info}` messages that include source/job context.
  - Risk/tradeoffs: Medium. Log volume may increase; may need level tuning to avoid noise.
  - Suggested validation: `rg -n "eprintln!" src/app/controller` should only match intentional CLI-entry paths; run `bash scripts/ci_local.sh`.

- [ ] 8) Remove `clippy::type_complexity` suppression from audio output stream construction
  - ROI/Effort: Medium / M
  - Why it matters: Type-complex tuple returns obscure ownership/lifetime intent in audio-core code.
  - Evidence:
    - File-level suppression at `src/audio/output.rs:1`.
    - `build_stream_with_state` returns a large tuple at `src/audio/output.rs:632`.
  - Recommended change: Replace tuple return with a named result struct (for stream + channels + flags) and remove suppression.
  - Risk/tradeoffs: Medium. Stream setup changes can affect callback wiring if ownership is altered.
  - Suggested validation: audio output unit tests (`src/tests/unit/audio_output_tests.rs`) + `cargo clippy --all-targets` + `bash scripts/ci_local.sh`.

- [ ] 9) Retire top app-level `too_many_arguments` suppressions using typed parameter objects
  - ROI/Effort: Medium / M
  - Why it matters: Wide signatures mask cohesion issues and make call sites brittle.
  - Evidence:
    - File-level suppressions remain in app paths such as:
      - `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs:1`
      - `src/app/controller/library/selection_export.rs:1`
      - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:1`
      - `src/app/controller/library/selection_edits/ops.rs:1`
      - `src/app/controller/library/source_cache_invalidator.rs:1`
  - Recommended change: Introduce typed request/context structs for highest-churn argument-heavy functions; remove suppressions incrementally.
  - Risk/tradeoffs: Medium. Signature updates can ripple across modules.
  - Suggested validation: targeted module tests + `cargo clippy --all-targets` + `bash scripts/ci_local.sh`.

- [ ] 10) Simplify analysis job-claim worker loop pacing into explicit policy helpers
  - ROI/Effort: Medium / M
  - Why it matters: Repeated inline sleep/wakeup policy inside long loops is hard to tune and reason about.
  - Evidence:
    - `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs` is 440 LOC.
    - Decoder/compute loops repeat wait/sleep branches (`Duration::from_millis(200)`, `sleep(Duration::from_millis(50))`) while mixing claim/queue/cancel behavior.
  - Recommended change: Extract pacing/backoff decisions into small policy helpers (or one policy object) so loop intent is explicit and testable.
  - Risk/tradeoffs: Medium. Worker responsiveness could change if pacing defaults shift.
  - Suggested validation: existing analysis job-claim tests + targeted pacing tests + `bash scripts/ci_local.sh`.

- [ ] 11) Split oversized native-bridge tests into focused test modules by concern
  - ROI/Effort: Low / M
  - Why it matters: Large mixed test files slow navigation and make regressions harder to triage.
  - Evidence:
    - `src/app_core/native_bridge/tests.rs` is 1071 LOC with many distinct concerns (projection keys, action reduction, cache counters, dirty segments).
  - Recommended change: Partition into submodules under `src/app_core/native_bridge/tests/` (for example `projection_keys`, `dirty_segments`, `action_reduction`, `metrics`).
  - Risk/tradeoffs: Low-medium. Mostly file movement with import-path churn.
  - Suggested validation: `cargo test -p sempal app_core::native_bridge` + `bash scripts/ci_local.sh`.

- [ ] 12) Add architecture docs for background job dispatch and file-op recovery contracts
  - ROI/Effort: Low / S
  - Why it matters: These subsystems have non-trivial sequencing assumptions that are currently implicit in code.
  - Evidence:
    - No direct docs surfaced for these internals from `rg -n "delete recovery|background job polling|file ops journal|source move" docs manual`.
  - Recommended change: Add concise docs under `docs/plans/active/` (or architecture section) describing invariants, stage contracts, and failure handling boundaries.
  - Risk/tradeoffs: Low. Documentation-only change.
  - Suggested validation: docs link checks via existing CI (`bash scripts/ci_local.sh`).

## Progress Log

- 2026-03-04: Phase 1 refreshed from current code state; waiting for explicit user confirmation before Phase 2.
- 2026-03-04: Completed item 1 (stale/under-budget allowlist entries removed; report now shows `missing=0`, `ok=0`).
- 2026-03-04: Completed item 2 (folder-move DB-failure test uses explicit lock acquire/release signaling instead of sleep-based timing).
