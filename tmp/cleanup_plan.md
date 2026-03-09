# Cleanup Plan (ROI Ranked)

Generated: 2026-03-09 (UTC)
Phase: 2 active
Status legend: `[ ]` pending, `[x]` done
Canonical local CI command: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Ordered Backlog

- [x] 1) Repair stale file-size allowlist entries and remove no-longer-needed exemptions
  - ROI/Effort: High / S
  - Why it matters: Stale allowlist rows hide real budget regressions and create maintenance noise in every file-size audit.
  - Evidence:
    - `docs/file_size_budget_allowlist.txt` contained stale and now-under-budget rows before the 2026-03-04 cleanup pass.
    - The old report showed missing and removable entries, which made file-size guardrails less trustworthy.
  - Recommended change: Remove stale/missing allowlist rows, prune all now-under-budget rows, and keep only active over-budget exceptions.
  - Risk/tradeoffs: Low. Only guardrail metadata changes.
  - Suggested validation: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1` and `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `8bb36ecc`

- [x] 2) Make folder-move DB-failure regression test deterministic (remove timing race)
  - ROI/Effort: High / S
  - Why it matters: A flaky core file-op test blocks CI confidence and wastes iteration time.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` previously used sleep-based lock timing in `folder_sample_move_db_write_failure_rolls_back_source_and_keeps_journal_for_recovery`.
    - The pre-fix test relied on long sleeps and timeout-based ordering instead of explicit synchronization.
  - Recommended change: Replace sleep-based lock timing with deterministic synchronization and guaranteed release scope.
  - Risk/tradeoffs: Low-medium. Test harness changes must preserve intended DB-failure semantics.
  - Suggested validation: targeted repeated test runs plus `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `fa24a4ff`

- [x] 3) Split background-job polling into focused message-router and handler modules with behavior tests
  - ROI/Effort: High / L
  - Why it matters: Job polling is a high-change integration hotspot; the old monolith shape increased coupling and regression risk.
  - Evidence:
    - `src/app/controller/library/background_jobs/polling.rs` previously combined routing, per-domain handlers, and helper logic in one 600+ LOC module.
    - Coverage was concentrated on helper predicates instead of end-to-end handler behavior before the split.
  - Recommended change: Extract routing plus per-domain handlers and add stale-message/progress/state tests.
  - Risk/tradeoffs: Medium-high. Dispatch rewiring can break subtle progress/state behavior.
  - Suggested validation: targeted background-job tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-06 (UTC) - `sempal` commit `4a4c1098`

- [x] 4) Expand file-op journal recovery coverage and document stage contracts
  - ROI/Effort: High / M
  - Why it matters: `file_ops_journal` is the durability boundary for crash recovery across source moves/copies; partial-stage mistakes can silently corrupt DB/file reconciliation.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 793 LOC.
    - The module exposes critical helpers and state transitions at `staged_relative_for_target` (`:190`), `insert_entry` (`:202`), `update_stage` (`:246`), `list_entries` (`:280`), and `reconcile_pending_ops` (`:436`).
    - Only four local tests exist (`:624`, `:691`, `:745`, `:782`), mostly happy-path move/copy recovery plus malformed-row cleanup.
    - There is no dedicated documentation in `docs/**` describing `Intent -> Staged -> TargetDb -> SourceDb` recovery semantics.
  - Recommended change: Add a focused test matrix for same-source move, cross-source move, copy, `TargetDb`/`SourceDb` partial completion, and missing staged-file scenarios; add a short design note documenting stage invariants and rollback expectations.
  - Risk/tradeoffs: Medium. Recovery tests touch real FS/SQLite state and can become brittle if setup is not explicit.
  - Suggested validation: targeted journal tests, repeated runs for the new matrix, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `2e3c35f2`

- [x] 5) Separate folder-delete recovery journal logic, recovery execution, and UI application; add a recovery matrix
  - ROI/Effort: High / L
  - Why it matters: Folder-delete recovery is safety-critical and currently hard to reason about because journal persistence, recovery policy, and controller/UI application live together.
  - Evidence:
    - `src/app/controller/library/source_folders/delete_recovery.rs` is 645 LOC.
    - One module currently owns staging/journaling (`stage_folder_for_delete` at `:109`), rollback (`rollback_staged_folder` at `:154`), bulk recovery (`recover_staged_deletes` at `:176`), and controller/UI application (`AppController::apply_folder_delete_recovery_report` starting at `:209`).
    - The file has only one local test (`:677`), which covers `unique_restore_path` but not journal stages or recovery outcomes.
    - Aside from `docs/plans/active/cleanup_architecture_note.md`, there is no concrete doc describing delete journal stages or when restore vs finalize is expected.
  - Recommended change: Split the module into focused journal/recovery/controller-apply pieces, add tests for `Intent`, `Staged`, `DbCommitted`, unjournaled staged folders, and restore collision handling, and document the recovery contract.
  - Risk/tradeoffs: Medium-high. Any refactor here must preserve crash-recovery behavior exactly.
  - Suggested validation: targeted delete-recovery tests, existing folder/file-op suites, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `cbc3c480`

- [x] 6) Refactor the source-move worker pipeline around explicit transactional stages and broaden failure-path tests
  - ROI/Effort: High / M
  - Why it matters: Source moves mutate filesystem state, DB state, and journal state in one workflow; sparse test coverage leaves cancellation and partial-failure behavior under-specified.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` is 672 LOC.
    - The file mixes request collection, background execution, controller apply, and stage helpers in one module, with the main worker flow centered around `run_source_move_task`.
    - Only two local tests exist (`:597` and `:654`), covering browser-row clearing and one progress-report case for a missing file.
    - The module still relies on journal-backed recovery infrastructure via `crate::sample_sources::db::file_ops_journal`, but there are no direct tests for mid-transaction DB/fs failures or cancellation ordering.
  - Recommended change: Split request prep, per-request transaction stages, and result application into separate helpers/modules; add tests for same-source no-op, target collision, DB write failure after stage, and cancellation after some requests complete.
  - Risk/tradeoffs: Medium. Transaction order bugs can show up only in failure paths, so test setup must stay explicit.
  - Suggested validation: targeted source-move tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `9615c7fe`

- [x] 7) Decompose the browser actions facade and tighten behavior tests around focus/selection invariants
  - ROI/Effort: High / L
  - Why it matters: Browser actions are central to daily UX; one wide controller facade makes it difficult to see which invariants are guaranteed by preview-vs-commit, anchor selection, and action-row logic.
  - Evidence:
    - `src/app/controller/library/wavs/browser_actions.rs` is 725 LOC.
    - The file mixes selected-path syncing, focus movement, view-window manipulation, range selection, prompt actions, and file/reveal helpers.
    - There is good higher-level coverage in `src/app/controller/tests/browser_actions.rs` (23 tests), but the production module still bundles multiple unrelated responsibilities and lacks module-level documentation describing key invariants.
  - Recommended change: Split the file into focused modules such as `focus_nav`, `selection_ranges`, and `row_actions`; add module docs for preview/commit semantics and keep the existing regression tests while adding anchor/autoscroll edge cases close to the extracted logic.
  - Risk/tradeoffs: Medium. Call-site churn is broad, but behavior can remain stable if extracted behind the same controller surface.
  - Suggested validation: browser action tests plus `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `2163c6eb`

- [x] 8) Add waveform load/cache lifecycle tests for cache hits, refresh reuse, and selection preservation
  - ROI/Effort: Medium / M
  - Why it matters: Waveform loading affects perceived responsiveness and view stability; subtle regressions here are easy to miss because the module couples cache use, selection preservation, transient extraction, and deferred metadata writes.
  - Evidence:
    - `src/app/controller/library/wavs/waveform_loading.rs` is 621 LOC.
    - Critical lifecycle entrypoints include `load_waveform_for_selection` (`:37`) and `finish_waveform_load_shared` (`:120`).
    - The file’s local tests only cover deferred metadata persistence deadlines (`:609` and `:636`).
    - Existing broader waveform controller tests in `src/app/controller/tests/waveform.rs` cover many edit/view behaviors, but not the cache-hit and refresh-preserve paths specific to `waveform_loading.rs`.
  - Recommended change: Add focused tests for cache-hit reuse, refresh-with-same-selection preservation, missing-file recovery after cached state exists, and transient/cache persistence interactions; add short docs on what “refresh” vs “selection load” guarantees.
  - Risk/tradeoffs: Medium. Test support may need more seam helpers to observe cache state without coupling to implementation details.
  - Suggested validation: targeted waveform-loading tests, broader waveform controller tests, then `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `b279884c`

- [x] 9) Document native-bridge projection cache, invalidation, and profiling contracts
  - ROI/Effort: Medium / S
  - Why it matters: The native bridge is now a critical retained-model subsystem; code and tests are extensive, but the design contract is mostly implicit, which raises change risk for future cleanup or perf work.
  - Evidence:
    - `src/app_core/native_bridge.rs` is 778 LOC, `src/app_core/native_bridge/projection_cache.rs` is 240+ LOC, `src/app_core/native_bridge/metrics.rs` is 863 LOC, and `src/app_core/native_bridge/tests.rs` is 1140 LOC.
    - The code defines important concepts such as `PendingWaveformActions` in `src/app_core/native_bridge.rs`, per-segment cache keys in `src/app_core/native_bridge/projection_cache.rs`, and multiple env-gated profiling/assertion modes in `src/app_core/native_bridge/metrics.rs`.
    - `docs/ARCHITECTURE.md` only provides an ownership-level note for `src/app_core`; there is no dedicated doc explaining retained projection segments, invalidation policy, or when bridge metrics/assertions should be used.
  - Recommended change: Add a dedicated developer note describing segment keys, invalidation boundaries, local-only pull fast paths, and profiling/assertion env vars; link it from `docs/README.md` and keep it synchronized with the existing test surface.
  - Risk/tradeoffs: Low. Docs-only, but the content must stay close to code reality.
  - Suggested validation: docs link checks plus a focused `cargo test -p sempal app_core::native_bridge` run and `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.
  - Completed: 2026-03-09 (UTC) - `sempal` commit `38ea54c5`

- [ ] 10) Standardize controller/worker error logging by replacing non-entrypoint `eprintln!` calls with structured tracing
  - ROI/Effort: Medium / M
  - Why it matters: Mixed stderr logging is inconsistent with the rest of the tracing-based diagnostics surface and makes error triage harder in production and automated runs.
  - Evidence:
    - Direct stderr logging remains in controller/worker paths such as:
      - `src/app/controller/library/analysis_jobs/pool/job_claim/db.rs:35`
      - `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs:105`
      - `src/app/controller/library/source_folders/delete_recovery.rs:308`
      - `src/app/controller/library/trash.rs:189`
      - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:226`
      - `src/app/controller/ui/file_ops.rs:97`
      - `src/app_core/native_bridge.rs:807`
    - Some of these are long-running worker loops or recovery paths where structured context would be more useful than raw stderr lines.
  - Recommended change: Replace internal `eprintln!` calls with `tracing::{info,warn,error}` and preserve plain stderr only at binary entrypoints/tools where immediate CLI output is intended.
  - Risk/tradeoffs: Medium. Log volume/levels may need tuning to avoid noisy hot paths.
  - Suggested validation: `git grep -n "eprintln!" -- src/app src/app_core` should only match intentional entrypoint-adjacent cases, then run `cargo clippy --workspace --all-targets` and `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 11) Retire remaining app-core wide-signature and type-complexity suppressions with typed parameter objects
  - ROI/Effort: Medium / M
  - Why it matters: The remaining suppressions mark exactly the places where signatures and tuple-shaped return values are obscuring ownership and responsibility boundaries.
  - Evidence:
    - File-level suppressions remain in:
      - `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs:1`
      - `src/app/controller/library/analysis_jobs/pool/job_claim/db.rs:1`
      - `src/app/controller/library/selection_edits/ops.rs:1`
      - `src/app/controller/library/selection_export.rs:1`
      - `src/app/controller/library/source_cache_invalidator.rs:1`
      - `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:1`
      - `src/audio/output.rs:1`
      - `src/waveform/render.rs:1`
    - Several of these areas also appear in the size audit, which suggests the suppressions are masking real decomposition opportunities rather than one-off unavoidable signatures.
  - Recommended change: Introduce typed request/context structs or named result structs incrementally, starting with the highest-churn paths, and remove the suppressions as each area becomes explicit.
  - Risk/tradeoffs: Medium. Signature cleanup ripples across call sites and should be done in small batches.
  - Suggested validation: targeted module tests, `cargo clippy --workspace --all-targets`, and `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

- [ ] 12) Refresh stale architecture/tooling docs after the workspace split
  - ROI/Effort: Medium / S
  - Why it matters: The docs are now partially inconsistent with the actual package layout, which makes onboarding and future cleanup decisions harder.
  - Evidence:
    - `docs/ARCHITECTURE.md:29-30` still points to `src/bin/sempal-updater` and `src/bin/sempal-installer`, but those binaries now live under `apps/updater-helper` and `apps/installer`.
    - `docs/file_size_budget_allowlist.txt:52-53` still references the pre-workspace tool paths.
    - The root `README.md` build instructions still default to shipping-profile `cargo run --release`, while the local iteration path now also includes `cargo run-fast` in `docs/build_speed.md`.
  - Recommended change: Refresh the architecture/build docs to reflect the workspace members and local fast-run lane, and clean stale tool paths from supporting docs/allowlists.
  - Risk/tradeoffs: Low. Documentation/tooling metadata only.
  - Suggested validation: docs/link checks plus `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`.

## Progress Log

- 2026-03-04: Phase 1 cleanup backlog first refreshed from current code state; waiting for explicit user confirmation before Phase 2.
- 2026-03-04: Completed item 1 (stale/under-budget allowlist entries removed; file-size guardrails cleaned).
- 2026-03-04: Completed item 2 (folder-move DB-failure test now uses explicit synchronization instead of timing sleeps).
- 2026-03-06: Completed item 3 (background-job polling split into focused modules; added stale-message and progress/state behavior tests).
- 2026-03-09: Cleanup backlog re-audited after the workspace split, native-bridge perf work, and recent ASIO/build-speed changes; pending items re-ranked for current ROI.
- 2026-03-09: Completed item 4 (expanded file-op journal recovery matrix, documented stage contracts, and cleared prerequisite CI blockers so `scripts/ci_local.ps1` is green again).
- 2026-03-09: Completed item 5 (split folder-delete recovery into journal/recovery/controller-apply modules, added stage-matrix coverage, and documented the restore/finalize contract).
- 2026-03-09: Completed item 6 (split the source-move flow into plan, registration, result-application, and transactional worker modules; added collision, rollback, and cancellation-path tests).
- 2026-03-09: Completed item 7 (split browser actions into focus-navigation, selection, and row-action modules; added local anchor/autoscroll invariant coverage).
- 2026-03-09: Completed item 8 (added waveform cache/refresh lifecycle coverage for same-path refresh reuse and cache-backed reloads, plus explicit load-vs-refresh docs).
- 2026-03-09: Completed item 9 (documented retained projection segments, invalidation boundaries, and native-bridge profiling/assertion contracts in a dedicated developer note).
