# Improvement Audit Plan

Generated: 2026-03-17
Status: Phase 2 in progress. Implementation started on 2026-03-17 after explicit user confirmation.

## Scope

- This document is the current evidence-driven improvement backlog for the live tree.
- It supersedes the earlier completed execution record that previously lived at this path.
- Items are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations are limited to repository-supported improvements; speculative direction is excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as a realtime-oriented Rust sample triage and curation tool for local audio libraries.
- Maturity level: Explicitly documented. `README.md` labels the app early alpha and warns that file operations can modify or delete user data.
- Primary languages / frameworks / tooling: Explicitly documented. The repo is a Rust workspace with a vendored `radiant` runtime/UI layer under `vendor/radiant`; Windows validation flows use `scripts/*.ps1`.
- Repository shape: Explicitly documented. Domain/controller logic lives under `src/`; GUI/runtime ownership is split between `src/app_core`, `src/gui*`, and `vendor/radiant`.
- Architectural boundaries: Explicitly documented. `README.md`, `docs/ARCHITECTURE.md`, and `docs/design_principles.md` keep domain intent in `src` and GUI behavior/runtime internals in `vendor/radiant`.
- Test strategy: Strongly implied by code/docs. The tree favors deterministic Rust unit/module tests plus targeted controller/runtime integration coverage, with GUI desktop automation kept as a local/manual lane.
- Canonical validation commands: Explicitly documented. `AGENTS.md`, `README.md`, and `docs/README.md` point to `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1` as the standard Windows gates.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes realtime responsiveness, non-blocking execution, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is a focused creative tool, not a platform.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for fast listening, navigation, editing, and curation of local sample libraries.
- What the repo appears to be moving toward: Strongly implied by code/docs. More automated guardrails, tighter module ownership, and safer file-operation workflows around the `radiant` runtime path and controller boundaries.
- What is merely possible but unsupported: large action-surface rewrites, CI-hosted desktop automation, and broad domain-model splits driven only by file size.

## Ordered Backlog

### 1. [x] Refresh stale cleanup-hotspot and quality-score artifacts before using them for further prioritization

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the current planning artifacts still describe yesterday's tree and now re-surface already-completed work, which makes later prioritization less trustworthy.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` was generated at commit `5e165afe` and still reports six over-budget files, including `src/app/controller/library/background_jobs/scan.rs`.
  - `docs/INDEX.md` documents `scripts/audit_cleanup_hotspots.ps1` as the canonical ROI-planning snapshot generator.
  - `docs/QUALITY_SCORE.md` still says code-size discipline depends on a live allowlist and calls out "live file size allowlist debt".
  - `docs/file_size_budget_allowlist.txt` currently contains only comments and no live allowlist entries.
- Recommended change: regenerate `tmp/cleanup_audit_hotspots.md` from the current tree and update `docs/QUALITY_SCORE.md` so it reflects the post-merge guardrail state instead of the retired allowlist-debt narrative.
- Expected impact: future audits and cleanup passes stop re-ranking finished work and regain a trustworthy baseline.
- Risks / tradeoffs: meta-work only; low functional risk, but it should not silently hide any newly emerged hotspot.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: pending final execution-record sync
  - Assumption: the live quality score should continue to report code-size discipline as `3` because full-scan debt remains even though allowlist debt is gone.
  - Validation:
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
    - Blocked by environment: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` failed because Windows Application Control blocked `C:\Users\wsvas\.cargo\bin\cargo-nextest.exe` before tests started

### 2. [x] Add direct rollback and cancellation coverage for folder-level moves before refactoring the worker

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: folder moves mutate filesystem and database state in one correctness-sensitive path, but the direct tests currently cover only the happy path while the implementation contains many rollback and rejection branches.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs` has many early `FolderMoveResult` returns plus repeated rollback-on-error filesystem renames after DB failures.
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves.rs` currently contains one direct `folder_move_updates_db_entries` test for folder-level moves; comparable cancel/failure tests exist only for folder-sample moves.
  - `docs/file_ops_journal_recovery.md` and `docs/folder_delete_recovery.md` both emphasize conservative crash-recovery and durable stage boundaries for file operations.
- Recommended change: add focused tests for pre-cancel behavior, self/descendant target rejection, existing-destination rejection, and DB-write failure rollback so later structural cleanup has a direct safety net.
- Expected impact: materially lowers the risk of regressing a data-integrity path during the follow-on refactor.
- Risks / tradeoffs: some failure paths may require a deterministic test hook or DB lock setup similar to the existing folder-sample move tests.
- Dependencies: none; this should land before item 3.
- Suggested validation:
  - targeted `folder_move` tests with `cargo test folder_move -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: pending final execution-record sync
  - Assumption: folder-move failure coverage should accept any DB-write-stage failure message that still proves the folder rename rolled back and DB state stayed on the source path.
  - Validation:
    - Passed: `cargo test folder_move -- --test-threads=1`
    - Passed: `rustfmt --edition 2024 --check src\app\controller\ui\drag_drop_controller\drag_effects\folder_moves.rs`
    - Blocked by environment: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` failed because Windows Application Control blocked `C:\Users\wsvas\.cargo\bin\cargo-nextest.exe` before tests started

### 3. [x] Decompose `run_folder_move_task` into validation, filesystem move, DB rewrite, and rollback/result helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: one 285-line function still owns request validation, destination derivation, DB enumeration, filesystem mutation, metadata rewrite, rollback, progress reporting, and result construction in a correctness-sensitive drag/drop path.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs` is 298 lines, and `run_folder_move_task` spans nearly the whole file.
  - The function interleaves input validation, `SourceDatabase::open`, `list_files`, `std::fs::rename`, batched DB mutation, rollback, and final progress/result assembly.
  - The existing recovery notes in `docs/file_ops_journal_recovery.md` and `docs/folder_delete_recovery.md` make these filesystem/database boundaries explicitly important.
- Recommended change: keep the public `run_folder_move_task` entrypoint and `FolderMoveResult` surface stable while extracting focused helpers for request validation/new-path derivation, source entry collection, filesystem rename/rollback, and DB metadata rewrite.
- Expected impact: the highest-risk move path becomes easier to review, test, and evolve without changing behavior.
- Risks / tradeoffs: move ordering and rollback semantics must remain exactly stable; helper extraction should not obscure the durable boundary sequence.
- Dependencies: item 2
- Suggested validation:
  - targeted `folder_move` tests with `cargo test folder_move -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: pending final execution-record sync
  - Assumption: preserving the public `run_folder_move_task` surface and mirroring each durable boundary with one helper is safer than introducing new cross-module abstractions.
  - Validation:
    - Passed: `cargo test folder_move -- --test-threads=1` before the final formatting-only patch to the same file
    - Passed: `rustfmt --edition 2024 --check src\app\controller\ui\drag_drop_controller\drag_effects\folder_moves\worker\folder_move_task.rs`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
    - Blocked by environment: rerunning `cargo test --lib folder_move -- --test-threads=1` after the formatting-only patch failed because Windows Application Control blocked `target\debug\deps\sempal-*.exe` before execution
    - Blocked by environment: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` failed because Windows Application Control blocked `C:\Users\wsvas\.cargo\bin\cargo-nextest.exe` before tests started

### 4. [x] Split folder-sample move execution by staged-move preparation, DB commit/journal updates, and finalize/report helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the folder-sample move worker already has useful coverage, but one large function still mixes per-request validation, staged-move preparation, DB writes, journal stage updates, finalize rename, and progress reporting.
- Evidence:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs` is 285 lines and `run_folder_sample_move_task` is the dominant branch-heavy function in the module.
  - The function owns `load_sample_move_metadata`, `prepare_staged_move`, DB batch writes, journal stage transitions, filesystem finalize rename, journal cleanup, and progress updates.
  - `docs/file_ops_journal_recovery.md` documents strict stage sequencing for this family of operations.
- Recommended change: preserve `run_folder_sample_move_task` as the orchestration seam while extracting per-request helpers or a small context type for validation/preparation, DB commit + stage transitions, and finalize/report cleanup.
- Expected impact: reduces controller/file-op complexity in a path that still carries many failure branches.
- Risks / tradeoffs: the staged-file and journal contract is sensitive; helper boundaries should mirror the durable state transitions rather than inventing abstraction layers.
- Dependencies: item 3
- Suggested validation:
  - targeted `folder_sample_move` and `folder_move` tests with `cargo test folder_move -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: pending final execution-record sync
  - Assumption: reusing the existing staged-move transaction pattern from cross-source sample moves is safer than inventing a separate folder-sample orchestration model.
  - Validation:
    - Passed: `cargo test folder_move -- --test-threads=1`
    - Passed: `rustfmt --edition 2024 --check src\app\controller\ui\drag_drop_controller\drag_effects\folder_moves\worker.rs src\app\controller\ui\drag_drop_controller\drag_effects\folder_moves\worker\folder_sample_move_task.rs`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
    - Blocked by environment: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` failed because Windows Application Control blocked `C:\Users\wsvas\.cargo\bin\cargo-nextest.exe` before tests started

### 5. [x] Replace the `process_batch_work` argument blob with a context struct and remove the remaining suppression

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters: one remaining `clippy::too_many_arguments` suppression sits on an analysis-worker helper that already has a clear shared-state shape, which makes this a contained cleanup with low behavioral risk.
- Evidence:
  - `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker/execution.rs` has `#[allow(clippy::too_many_arguments)]` on `process_batch_work`.
  - The function takes stable shared inputs (`allowed_source_ids`, `log_jobs`, `settings`, `decode_queue`) plus mutable output accumulators (`decoded_batches`, `immediate_jobs`) that naturally group into a context.
  - The broader file was recently decomposed; this is now one of the visible remaining local cleanup seams.
- Recommended change: introduce a focused batch-processing context/accumulator type, thread it through `process_batch`, and remove the suppression without changing job-routing behavior.
- Expected impact: clearer ownership in the compute worker and one less lingering suppression in a hot path.
- Risks / tradeoffs: low, but the refactor should avoid adding indirection on the decode/analysis fast path without need.
- Dependencies: none
- Suggested validation:
  - targeted analysis-job tests with `cargo test analysis_jobs -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Execution notes:
  - Date: 2026-03-17
  - Commit: pending final execution-record sync
  - Assumption: a single mutable batch-processing context is the smallest cleanup that removes the suppression without changing queueing or connection-reuse behavior.
  - Validation:
    - Passed: `rustfmt --edition 2024 --check src\app\controller\library\analysis_jobs\pool\job_claim\compute_worker\execution.rs`
    - Passed: `cargo test analysis_jobs -- --test-threads=1`
    - Passed: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
    - Blocked by environment: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` failed because Windows Application Control blocked `C:\Users\wsvas\.cargo\bin\cargo-nextest.exe` before tests started

## Open Questions / Missing Definitions

### [!] 1. Should `vendor/radiant/src/app/actions/mod.rs` remain one intentionally centralized compatibility surface?

- Evidence:
  - The module docs explicitly say `UiAction` is intentionally centralized and should stay broad unless a concrete contract mismatch appears.
  - The file is still one of the larger live Rust modules, so size-only heuristics will keep nominating it unless that intent remains visible.
- Why this matters: cleanup work could otherwise split a deliberate runtime/host-bridge compatibility seam for the wrong reason.
- Affected files/modules: `vendor/radiant/src/app/actions/mod.rs`, runtime action routing, host bridge, automation action catalog.
- Risk if guessed incorrectly: premature splitting could destabilize one inspectable action surface shared across runtime and host layers.
- Most conservative provisional assumption: keep `UiAction` centralized and improve internal organization around it only if a concrete routing or ownership mismatch appears.

### [!] 2. Should `src/selection/range.rs` continue to keep waveform geometry, fades, and gain math together?

- Evidence:
  - The module docs explicitly say the dense file intentionally preserves one waveform-editing domain model.
  - The file remains large enough to attract size-driven cleanup suggestions even though the current docs argue for cohesion.
- Why this matters: splitting a cohesive domain contract just to reduce size would add churn without clear behavioral benefit.
- Affected files/modules: `src/selection/range.rs`, waveform selection preview, fade handles, destructive edit flows.
- Risk if guessed incorrectly: over-splitting could scatter one stable normalized-selection contract across several low-value helpers.
- Most conservative provisional assumption: keep the module cohesive unless a clearer subdomain or ownership boundary emerges.

## Rejected Ideas

### [-] 1. Reopen the old background-job and audio-options split items

- Why it was considered: stale audit artifacts still list several of those files as active hotspots.
- Why it was rejected: the live tree has already absorbed those changes, the current plan file at this path had become a completed execution record, and the full-scan file-size guardrail is currently green.
- What evidence was missing: any live guardrail failure or current-tree hotspot evidence showing those exact items are still open.

### [-] 2. Split `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs` immediately

- Why it was considered: it remains a relatively large runtime text-entry file.
- Why it was rejected: the module docs explicitly say it intentionally centralizes one shared text-entry flow for browser search and waveform BPM editing.
- What evidence was missing: a concrete ownership or behavioral conflict that the current single-flow structure is causing.

### [-] 3. Promote AIV desktop automation into normal CI now

- Why it was considered: the repository continues to invest in GUI automation infrastructure and local AIV workflows.
- Why it was rejected: current docs still treat AIV as a local/manual lane rather than a CI-ready contract.
- What evidence was missing: repeated proof that focus, foregrounding, and timing are deterministic enough across CI environments.
