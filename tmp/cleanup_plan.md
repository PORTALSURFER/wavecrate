# Cleanup Plan (ROI Ranked)

Generated: 2026-03-04 (UTC)
Phase: 1 audit complete; Phase 2 pending explicit user confirmation
Status legend: `[ ]` pending, `[x]` done

## Ordered Backlog

- [x] 1) Extract native bridge waveform action reduction/flush pipeline into staged helpers with focused tests
  - ROI/Effort: High / M
  - Why it matters: Waveform input (seek/cursor/selection/zoom) is on a high-frequency path, but reduction, coalescing, cache invalidation, and dirty propagation are tightly coupled in one block.
  - Evidence:
    - `src/app_core/native_bridge.rs` is 695 LOC.
    - `flush_pending_waveform_actions` at `src/app_core/native_bridge.rs:345`.
    - `reduce_action` at `src/app_core/native_bridge.rs:595` duplicates flow around immediate/deferred action handling.
  - Recommended change: Split into explicit stages (reduce, apply, invalidate, mark dirty), keep ordering semantics unchanged, and add table-driven tests for mixed queued action sets.
  - Risk/tradeoffs: Medium-high. Ordering regressions can break interaction semantics or undo performance wins.
  - Suggested validation: targeted native-bridge action-reduction tests + `bash scripts/ci_local.sh`.
  - Completed: 2026-03-04 (UTC) - `sempal` commit `80165521`

- [ ] 2) Split playback audio loader into explicit execution stages and isolated telemetry state
  - ROI/Effort: High / L
  - Why it matters: Audio load latency correctness is high-impact; one large function currently mixes IO, decode, sanitization, stretch, stale gating, and accounting.
  - Evidence:
    - `src/app/controller/playback/audio_loader.rs` is 693 LOC.
    - `load_audio_inner` spans from `src/app/controller/playback/audio_loader.rs:368`.
    - File-level telemetry globals/counters around `audio_loader.rs:23-164` increase coupling.
  - Recommended change: Extract `io`, `decode`, `stretch`, and `finalize` stages plus a telemetry helper module, preserving request-id/stale-drop semantics.
  - Risk/tradeoffs: Medium-high. Stage reorder mistakes can cause stale data application or audio behavior drift.
  - Suggested validation: stale-stage table tests + existing audio_loader tests + `bash scripts/ci_local.sh`.

- [ ] 3) Refactor source-move worker into staged operations with unified completion/failure progress handling
  - ROI/Effort: High / M
  - Why it matters: Move workflows are failure-prone and currently branch-heavy; repeated completion/progress code makes correctness auditing difficult.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` is 607 LOC.
    - `run_source_move_task` starts at `source_moves.rs:297` and contains repeated `completed += 1` / progress-report branches.
  - Recommended change: Introduce focused per-stage helpers and one shared completion/failure accounting path.
  - Risk/tradeoffs: Medium. Error-path behavior must remain exact for rollback/reporting parity.
  - Suggested validation: failure-injection tests across target-db open/write/delete/rename paths + `bash scripts/ci_local.sh`.

- [ ] 4) Split native shell projection facade into narrower projection modules with stable re-exports
  - ROI/Effort: High / L
  - Why it matters: `native_shell.rs` remains a large mixed-responsibility projection surface, slowing review and increasing edit collision risk.
  - Evidence:
    - `src/app_core/native_shell.rs` is 758 LOC.
    - Core mixed projection entry points: `project_app_model` (`:103`), `project_update_model` (`:324`), `project_confirm_prompt_model` (`:408`), `project_sources_model` (`:567`).
  - Recommended change: Move update/prompt/sources/browser projection logic into dedicated module files under `src/app_core/native_shell/`, keeping public API unchanged.
  - Risk/tradeoffs: Medium. Projection wiring mistakes can regress UI model parity.
  - Suggested validation: existing native_shell parity tests + targeted projection regression tests + `bash scripts/ci_local.sh`.

- [ ] 5) Harden file-ops journal reconciliation by removing silent malformed-entry drops and panic-style expect paths
  - ROI/Effort: High / M
  - Why it matters: Journal reconciliation is safety-critical; silently skipping malformed rows and panic-style assumptions weaken recoverability.
  - Evidence:
    - `src/sample_sources/db/file_ops_journal.rs` is 615 LOC.
    - `list_entries` at `file_ops_journal.rs:247` drops invalid paths via `Ok(None)` logic.
    - `reconcile_entry` at `file_ops_journal.rs:343` includes `expect("checked staged path")` at `:359`.
  - Recommended change: Return explicit malformed-entry outcomes (or quarantined errors), split reconciliation into phase helpers, remove `expect` from runtime path.
  - Risk/tradeoffs: Medium. Stricter handling may surface pre-existing journal corruption more visibly.
  - Suggested validation: malformed-entry + reconcile-matrix tests (staged/target/source combinations) + `bash scripts/ci_local.sh`.

- [ ] 6) Extract `toggle_loop` policy flow into smaller decision helpers with scenario coverage
  - ROI/Effort: Medium / M
  - Why it matters: Loop behavior touches playback state, selection semantics, DB writes, and UI; current all-in-one flow is hard to reason about.
  - Evidence:
    - `src/app/controller/playback/transport.rs` is 708 LOC.
    - `toggle_loop` spans dense branching at `transport.rs:188-303`.
  - Recommended change: Split policy into explicit branches (enable, disable, defer-disable, restart) and isolate side effects into named helpers.
  - Risk/tradeoffs: Medium. Sequence changes can alter audible behavior and loop state timing.
  - Suggested validation: scenario tests for playing/not-playing and with/without selection + `bash scripts/ci_local.sh`.

- [ ] 7) Split `wavs.rs` controller facade by responsibility (cache, selection, metadata, browser actions)
  - ROI/Effort: Medium / L
  - Why it matters: The wavs controller facade remains very large and serves many unrelated concerns, increasing coupling and churn risk.
  - Evidence:
    - `src/app/controller/library/wavs.rs` is 638 LOC.
    - File contains broad method surface (cache refresh, selection, metadata writes, browser focus/sort helpers).
  - Recommended change: Partition by responsibility into submodules while preserving `AppController` API surface.
  - Risk/tradeoffs: Medium. Large call-site movement can create merge friction.
  - Suggested validation: controller wav/browser selection tests + `bash scripts/ci_local.sh`.

- [ ] 8) Defer non-critical DB metadata writes from immediate waveform load path
  - ROI/Effort: Medium / M
  - Why it matters: The waveform load path includes synchronous metadata write/open operations that can add interaction jitter.
  - Evidence:
    - `src/app/controller/library/wavs/waveform_loading.rs` is 506 LOC.
    - Metadata write/update block around `waveform_loading.rs:314-390`.
    - Additional DB open/read for BPM near `waveform_loading.rs:426-442`.
  - Recommended change: Keep waveform render/load path focused on decode/display and queue metadata persistence asynchronously where safe.
  - Risk/tradeoffs: Medium. Deferred writes introduce eventual consistency windows.
  - Suggested validation: interaction latency sanity checks + metadata convergence integration tests + `bash scripts/ci_local.sh`.

- [ ] 9) Replace remaining file-level `clippy::too_many_arguments` suppressions in core hotspots with typed parameter structs
  - ROI/Effort: Medium / M
  - Why it matters: File-level suppressions hide call complexity and reduce signature clarity in frequently edited modules.
  - Evidence:
    - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:1`
    - `src/app/controller/library/wavs/waveform_loading.rs:1`
    - `src/sample_sources/db/file_ops_journal.rs:1`
  - Recommended change: Convert top 2-3 argument-heavy functions per file to typed input structs and narrow/retire suppressions incrementally.
  - Risk/tradeoffs: Medium. Signature changes can cascade through call sites.
  - Suggested validation: `cargo clippy --all-targets` + targeted domain tests + `bash scripts/ci_local.sh`.

- [ ] 10) Close crate-visible documentation gaps in native shell/bridge high-churn APIs
  - ROI/Effort: Low / S
  - Why it matters: Cross-module projection helpers are reused widely but some key crate-visible APIs still lack explicit intent/constraints docs.
  - Evidence:
    - Undocumented crate-visible entry points in `src/app_core/native_shell.rs`, including `project_app_model` (`:103`), `project_motion_model` (`:259`), and `selected_column_index` (`:555`).
  - Recommended change: Add concise doc comments for what/why/constraints on crate-visible projection helpers.
  - Risk/tradeoffs: Low. Documentation-only change.
  - Suggested validation: `RUSTDOCFLAGS='-D warnings' cargo doc -p sempal --no-deps` + `bash scripts/ci_local.sh`.

- [ ] 11) Add focused tests for profile message formatting and projection-cache counters in metrics paths
  - ROI/Effort: Low / M
  - Why it matters: Bridge metrics logic is feature-gated and structurally complex; current tests emphasize helper math over output-shape regression coverage.
  - Evidence:
    - `src/app_core/native_bridge/metrics.rs` is 834 LOC with format/publish logic centered around `format_bridge_profile_message` near `metrics.rs:398`.
  - Recommended change: Add tests asserting expected field presence and stable formatting for key profile message outputs under metrics-enabled builds.
  - Risk/tradeoffs: Low-medium. Feature-gated test setup can be brittle without careful scaffolding.
  - Suggested validation: `cargo test -p sempal --features native-bridge-metrics app_core::native_bridge::metrics` + `bash scripts/ci_local.sh`.

- [ ] 12) Consolidate duplicated BPM display formatting into one shared helper
  - ROI/Effort: Low / S
  - Why it matters: Duplicate formatting rules increase drift risk and create small but recurring maintenance noise.
  - Evidence:
    - Formatting logic duplicates in `src/app/controller/playback/transport.rs:475-481` and `src/app/controller/library/wavs/waveform_loading.rs:449-455`.
  - Recommended change: Introduce a single shared formatting helper and cover integer/fractional/invalid BPM cases with unit tests.
  - Risk/tradeoffs: Low. Small call-site updates only.
  - Suggested validation: targeted formatting tests + `bash scripts/ci_local.sh`.

## Progress Log

- 2026-03-04: Phase 1 refreshed from current code state; awaiting explicit user confirmation before Phase 2 implementation.
- 2026-03-04: Completed item 1 (native bridge waveform action reduction/flush staging + mixed queue emission tests).
