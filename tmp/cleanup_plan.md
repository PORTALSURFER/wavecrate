# Cleanup Plan (ROI Ranked)

Generated: 2026-03-04 (UTC)
Phase: 1 audit complete; Phase 2 pending explicit user confirmation
Status legend: `[ ]` pending, `[x]` done

## Ordered Backlog

- [x] 1) Decompose `vendor/radiant` native shell state renderer into focused modules
  - ROI/Effort: High / L
  - Why it matters: The native shell state layer is still the largest single hotspot and mixes frame assembly, overlay construction, input/action helpers, and toolbar composition in one file, which raises regression risk and review overhead.
  - Evidence:
    - `vendor/radiant/src/gui/native_shell/state.rs` (4346 LOC).
    - `build_frame_with_style_into_with_motion_sinks` at line 1264 and `build_state_overlay_into` at line 2906 are very large core paths.
  - Recommended change: Extract `state/frame_build.rs`, `state/overlay.rs`, and `state/actions.rs` (or equivalent) with a thin facade in `state.rs`; preserve existing public entrypoints and behavior.
  - Risk/tradeoffs: Medium-high. Splitting render/input code can introduce subtle ordering regressions if boundaries are not exact.
  - Suggested validation: `cargo test --manifest-path vendor/radiant/Cargo.toml` then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `radiant` commit `e2b60e0d`

- [x] 2) Unify duplicated staged file-move transaction logic across drag/drop workers
  - ROI/Effort: High / M
  - Why it matters: Source moves and folder sample moves duplicate long staged move/journal/rollback sequences, making bug fixes and behavioral changes expensive and inconsistent.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:312` (`run_source_move_task`, 298-line span by audit script).
    - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:17` (`run_folder_sample_move_task`, 278-line span).
    - Repeated rollback + journal cleanup branches across both functions.
  - Recommended change: Introduce a shared staged move executor (plan/apply/rollback primitives) used by both workers, keeping job result types unchanged.
  - Risk/tradeoffs: Medium. Shared helper mistakes could affect both move paths simultaneously.
  - Suggested validation: drag/drop move tests, file-op journal recovery tests, `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `fc9241ed`

- [x] 3) Add dedicated tests for folder move worker cancellation and rollback semantics
  - ROI/Effort: High / M
  - Why it matters: Non-trivial file operation code currently lacks local coverage in `folder_moves`, leaving cancellation/journal-stage edge cases underprotected.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs` contains core move executors at lines 17 and 295.
    - No `mod tests`/`#[test]` matches in `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/*.rs` from audit scan.
  - Recommended change: Add focused tests for cancel-before-start, staged rename failure rollback, DB update failure rollback, and journal cleanup on success/failure.
  - Risk/tradeoffs: Low-medium. Test setup will require deterministic temp-dir and DB fixtures.
  - Suggested validation: run new folder move tests directly, then `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `9ce005e8`

- [x] 4) Split `IssueTokenStore` into backend/key-management/storage modules
  - ROI/Effort: High / M
  - Why it matters: Keyring operations, fallback key lifecycle, encrypted file IO, env parsing, and large test blocks are tightly coupled in one file, making security-sensitive changes harder to reason about.
  - Evidence:
    - `src/issue_gateway/token_store.rs` (1085 LOC).
    - Global mutable fallback state at lines 20-21.
    - Mixed fallback key lifecycle (`ensure_fallback_key` at line 206) and IO paths (`fallback_get` at 347, `fallback_set` at 392).
    - Inline tests start at line 653.
  - Recommended change: Extract modules for `keyring_backend`, `fallback_key`, `fallback_store`, and `crypto`; keep `IssueTokenStore` as orchestration facade.
  - Risk/tradeoffs: Medium. Storage compatibility/migration behavior must stay byte-for-byte compatible.
  - Suggested validation: token-store unit tests (including corruption and migration cases) and `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `67be23f2`

- [x] 5) Reduce projection cache complexity by splitting derive/materialize/probe layers
  - ROI/Effort: High / M
  - Why it matters: Retained projection cache logic combines key derivation, segment materialization, dirty-segment policy, and benchmark probes in one dense module, increasing maintenance cost in a performance-critical path.
  - Evidence:
    - `src/app_core/native_bridge/projection_cache.rs` (1064 LOC).
    - `resolve_or_project_with_derived` at line 547, `build_projection_cache_key` at line 641, and probe helper `run_rebuild_cause_probe_iters` at lines 985-986.
    - `#[allow(clippy::too_many_arguments)]` on probe helper at line 985.
  - Recommended change: Split into `projection_key.rs`, `segment_materialize.rs`, and `probe_metrics.rs`; replace high-arity probe inputs with typed context structs.
  - Risk/tradeoffs: Medium. Any key derivation drift can impact UI invalidation correctness.
  - Suggested validation: projection cache unit tests + perf guard checks + `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `f66c7acf`

- [x] 6) Refactor `ControllerJobs` state and worker launch boilerplate
  - ROI/Effort: Medium / M
  - Why it matters: `ControllerJobs` maintains many independent in-progress/cancel fields and repeated `thread::spawn` launch patterns, which increases coupling and chance of state-flag inconsistencies.
  - Evidence:
    - `src/app/controller/jobs.rs` (1171 LOC), `ControllerJobs` starts at line 562.
    - Repeated launcher blocks at lines 863, 990, 1026, 1051, 1077, and 1091.
  - Recommended change: Introduce grouped task-state structs and a shared spawn/forward helper for one-shot background tasks; keep message protocol stable.
  - Risk/tradeoffs: Medium. Async lifecycle regressions can occur if clear/start state transitions are altered incorrectly.
  - Suggested validation: controller job tests + integration smoke for scan/file-op/update workflows + `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `71a6ae28`

- [x] 7) Continue splitting `wavs.rs` façade by responsibility boundaries
  - ROI/Effort: Medium / M
  - Why it matters: The file still combines metadata preloading, DB mutation helpers, cache reconciliation, selection path rewrites, and browser-facing actions, slowing safe iteration in sample-browser behavior.
  - Evidence:
    - `src/app/controller/library/wavs.rs` (938 LOC).
    - Mixed concerns in `preload_bpm_values_for_paths` (130), `normalize_and_save_for_path` (252), `rewrite_db_entry_for_source` (356), `update_cached_entry` (480), and browser-facing APIs (`find_similar_for_visible_row` at 752, `preview_sample_by_id` at 846).
  - Recommended change: Extract `wavs/entry_mutation.rs` and `wavs/metadata_cache.rs` helpers; keep selection/search APIs in current façade.
  - Risk/tradeoffs: Medium-low. Broad call-site updates can create minor merge friction.
  - Suggested validation: browser selection/search tests and `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `2f1f10e7`

- [x] 8) Move heavy inline test modules out of production source files
  - ROI/Effort: Medium / S
  - Why it matters: Large inline test sections inflate production files and hide production-only responsibilities.
  - Evidence:
    - `src/audio/output.rs` production logic with inline tests at `mod tests` line 686.
    - `src/sample_sources/db/mod.rs` production logic with inline tests at `mod tests` line 374.
  - Recommended change: Move tests to sibling `tests.rs` (or submodule tree) and keep production modules focused.
  - Risk/tradeoffs: Low. Mostly structural movement, minimal runtime risk.
  - Suggested validation: targeted crate tests for moved modules and `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `8a314cdd`

- [ ] 9) Remove or justify `dead_code` suppressions in source DB write API
  - ROI/Effort: Medium / S
  - Why it matters: Suppressed dead code in production paths can hide stale or duplicate APIs and increases maintenance burden.
  - Evidence:
    - `src/sample_sources/db/write.rs` has `#[allow(dead_code)]` on `upsert_file` (line 10) and `set_tag` (line 50).
    - Batch equivalents exist in the same file (`SourceWriteBatch::upsert_file` line 150 and `SourceWriteBatch::set_tag` line 305).
  - Recommended change: Remove unused wrapper methods if truly unused, or replace with explicit test-only gating and rationale.
  - Risk/tradeoffs: Low. Potential call-site updates if wrappers are still needed.
  - Suggested validation: `cargo clippy --all-targets` and `bash scripts/ci_local.sh`.

- [ ] 10) Close documentation gaps for public job DTOs in controller jobs module
  - ROI/Effort: Medium / S
  - Why it matters: Several externally consumed job/result structs in `jobs.rs` lack doc comments, making async protocol intent and constraints harder to maintain.
  - Evidence:
    - Undocumented public structs include `IssueGatewayJob` (line 133), `IssueGatewayPollJob` (139), `IssueGatewayCreateResult` (144), `IssueGatewayAuthResult` (152), and `IssueTokenSaveJob` (158).
  - Recommended change: Add concise rustdoc for purpose, ownership, and constraints for each public job/result type.
  - Risk/tradeoffs: Low. Documentation-only change.
  - Suggested validation: `RUSTDOCFLAGS='-D warnings' cargo doc -p sempal --no-deps` and `bash scripts/ci_local.sh`.

- [ ] 11) Add a repeatable cleanup hotspot audit script for future passes
  - ROI/Effort: Low / S
  - Why it matters: This audit currently depends on ad-hoc shell commands; a deterministic script reduces drift and speeds future cleanup planning.
  - Evidence:
    - Current findings were produced via manual `wc`, `rg`, and custom `awk` calls with no canonical script entrypoint in `scripts/`.
  - Recommended change: Add `scripts/audit_cleanup_hotspots.sh` that emits file-size/function-length/suppression/test-density snapshots into `tmp/`.
  - Risk/tradeoffs: Low. Small maintenance burden to keep thresholds and reports useful.
  - Suggested validation: run the script locally and verify output under version control exclusions, then `bash scripts/ci_local.sh`.

- [ ] 12) Add a cleanup architecture note linking debt items to module boundaries
  - ROI/Effort: Low / S
  - Why it matters: The cleanup queue spans controller, DB, and runtime layers; a short architecture note prevents future passes from reintroducing the same boundary violations.
  - Evidence:
    - Current debt spans `src/app/controller`, `src/sample_sources`, `src/app_core/native_bridge`, and `vendor/radiant` with repeated boundary-mixing patterns.
  - Recommended change: Add a concise doc under `docs/plans/active/` describing target boundaries and ownership for the top cleanup hotspots.
  - Risk/tradeoffs: Low. Documentation overhead only.
  - Suggested validation: docs consistency review and `bash scripts/ci_local.sh`.

## Progress Log

- 2026-03-04: Phase 1 audit complete; backlog refreshed and awaiting explicit user confirmation before Phase 2 implementation.
