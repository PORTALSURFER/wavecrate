# Improvement Audit Plan

- Generated: `2026-03-15`
- Status: `Phase 2 complete`
- Branch: `next`
- Audit baseline commit: `2b2054a5`
- Execution completion date: `2026-03-15`
- Canonical Windows validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

## Repository Context

- Project purpose: `sempal` is a realtime-oriented creative sample manager focused on exploration, auditioning, curation, and non-blocking interaction.
  - Intent label: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`
- Maturity level: mature, actively-guardrailed Rust workspace with local/CI validation, vendor UI framework ownership, and multiple completed execution plans.
  - Intent label: Strongly implied by code/docs
  - Evidence: `Cargo.toml`, `.github/workflows/ci.yml`, `docs/INDEX.md`, `docs/TEST.md`, `docs/plans/index.md`
- Primary languages/frameworks/tooling: Rust 2024 edition workspace, `vendor/radiant` native GUI framework, Vello/Winit runtime, rusqlite, nextest, PowerShell/Bash validation wrappers.
  - Intent label: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `docs/ARCHITECTURE.md`, `docs/TEST.md`
- Repository shape: root app plus installer/updater helper apps, support tools, `vendor/radiant`, `docs/`, `manual/`, and active/parked plan files under `docs/plans` and `tmp/`.
  - Intent label: Explicitly documented
  - Evidence: `Cargo.toml`, `docs/ARCHITECTURE.md`, `docs/README.md`
- Architectural boundaries: domain logic in `src/`, UI framework behavior in `vendor/radiant/`, app-core/native bridge in `src/app_core`, with legacy `src/app` still present but guarded against new coupling from non-legacy layers.
  - Intent label: Explicitly documented
  - Evidence: `docs/ARCHITECTURE.md`, `.github/workflows/ci.yml`, `docs/INDEX.md`
- Test strategy: fast local compile/check loop plus `ci_quick`, full `ci_local`, nextest for workspace tests, dedicated `vendor/radiant` tests, and a GUI contract/AIV stack that is intentionally not fully promoted to CI.
  - Intent label: Explicitly documented
  - Evidence: `docs/TEST.md`, `docs/gui_test_platform.md`, `.github/workflows/ci.yml`
- Documented priorities: responsiveness, non-blocking behavior, predictable interaction, deterministic hotkeys, GUI contract stability, and incremental cleanup of oversized modules.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`, `docs/gui_test_platform.md`, `docs/QUALITY_SCORE.md`, `docs/INDEX.md`
- Explicit non-goals: not a DAW replacement, not a cloud/social platform, not attention-capture software, and not a visually ornamental app at the expense of responsiveness.
  - Intent label: Explicitly documented
  - Evidence: `docs/design_principles.md`

## Ordered ROI Backlog

### [x] 1. Decompose `vendor/radiant/src/gui_runtime/native_vello/runtime_events.rs` by event family
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `dbdb2944` (`refactor(runtime): split native vello event families`)
- Assumption used: local module extraction around the existing runtime runner is the conservative boundary, not a runner-state redesign.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: this is a timing-sensitive native runtime entrypoint, and one broad event hub currently owns window lifecycle, pointer routing, keyboard/text-input behavior, redraw triggers, and idle scheduling. That makes hot-path changes riskier than they need to be.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events.rs` is `543` lines in the current tree.
  - The file implements `ApplicationHandler<RuntimeUserEvent>` and handles `resumed`, `window_event`, `user_event`, and `about_to_wait`.
  - `window_event` routes resize, pointer move, pointer press/release, wheel, modifiers, keyboard, and redraw logic in one match.
  - The same file also owns `handle_left_pointer_press`, `handle_right_pointer_press`, `handle_mouse_wheel`, and `handle_keyboard_input`.
  - Existing targeted runtime tests are mostly around `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer.rs` and `vendor/radiant/src/gui_runtime/native_vello/tests/runtime_core.rs`, not around the full keyboard/text-input or drag-session transition matrix.
- Recommended change: split event handling into focused modules for lifecycle/window events, pointer press/release routing, wheel routing, keyboard/text-input routing, and wait/redraw scheduling while keeping the external runner API and behavior unchanged.
- Expected impact: safer edits in the hottest runtime path, smaller review scope for input changes, and clearer ownership between scheduling and interaction logic.
- Risks / tradeoffs: event ordering and invalidation semantics are performance-sensitive; characterization tests need to move with the code before behavior changes.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 2. Split `vendor/radiant/src/gui_runtime/native_vello/runtime_input.rs` into cursor pacing, viewport sync, waveform drag, and pointer-finish modules
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Completed: `2026-03-15`
- Commit: `7646feac` (`refactor(runtime): split native vello input stages`)
- Assumption used: cursor pacing, viewport sync, drag-session mutation, and flush/finalization behavior can be separated without changing event ordering.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: this file is the immediate-input hot path for the native runtime and currently mixes browser viewport sync, cursor state, redraw pacing, browser and waveform scrollbar drags, waveform drag modes, map drag focus, volume drag flushing, and pending-input drain logic.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input.rs` is `557` lines.
  - The file owns `sync_browser_viewport_from_shell`, `process_cursor_move_immediately`, `process_browser_scrollbar_drag_immediately`, `process_waveform_scrollbar_drag_immediately`, `process_waveform_pan_drag_immediately`, `process_waveform_drag_immediately`, `process_selection_drag_immediately`, `process_map_focus_drag_immediately`, `finish_volume_drag`, `flush_pending_input`, and redraw pacing helpers.
  - `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer.rs` covers browser pointer and wheel behavior, but the file itself has no local `#[cfg(test)]` module and the surrounding tests do not directly cover text-input drag, map-drag de-duplication, or pending-volume/session finish behavior.
- Recommended change: isolate pure cursor/redraw pacing helpers, browser viewport sync, waveform drag-session handling, and pointer-finish/pending-input flush logic into separate modules and add direct tests for session-finalization edge cases.
- Expected impact: lower regression risk in interaction hot paths and better direct coverage for drag-session invariants.
- Risks / tradeoffs: drag-state cleanup bugs would be user-visible; any split should preserve current emission ordering exactly.
- Dependencies: item 1 is adjacent but not required
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::browser_pointer -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 3. Separate model-pull/dirty-resolution from encode/composition in `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene.rs`
- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: L
- Completed: `2026-03-15`
- Commit: `ba3ad7ee` (`refactor(runtime): split native vello scene rebuild stages`)
- Assumption used: bridge refresh resolution, fingerprint maintenance, and final composition are separable as long as the retained-scene contract and runner API stay unchanged.
- Validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::runtime_core -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: `rebuild_scene` is the core retained-render rebuild path. It currently mixes bridge pulls, dirty-segment interpretation, fingerprint maintenance, static segment rebuilds, overlay rebuilds, and final scene composition in one large control-flow hub.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene.rs` is `562` lines.
  - The file owns `cached_image_upload_blob`, static-segment fingerprint construction, cache fingerprint builders, `rebuild_static_segment_scenes`, and `rebuild_scene`.
  - `rebuild_scene` handles model pulls, motion-model fallback, segment revision refresh, overlay fingerprint checks, static rebuild routing, and final scene append composition in one method.
  - `vendor/radiant/src/gui_runtime/native_vello/tests/runtime_core.rs` directly covers cache reuse and dirty-graph helpers, but not the broader `rebuild_scene` branch matrix.
- Recommended change: keep the public runtime render surface stable while extracting focused helpers/modules for bridge refresh resolution, cache fingerprinting, static-segment rebuild planning, and final scene composition.
- Expected impact: clearer ownership in the most performance-sensitive render path and a more testable separation between data refresh and scene encoding.
- Risks / tradeoffs: this is a high-sensitivity path for both correctness and performance; the split must be behavior-preserving and heavily characterization-tested.
- Dependencies: none
- Suggested validation:
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests::runtime_core -- --test-threads=1`
  - `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 4. Consolidate waveform RGBA conversion ownership across the legacy controller and `app_core` native projection path
- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: S
- Completed: `2026-03-15`
- Commit: `909a4893` (`refactor(waveform): centralize native rgba translation`)
- Assumption used: only the pure RGBA translation contract should be centralized now, without moving broader waveform-render ownership across layers.
- Validation:
  - `cargo test waveform_image_to_native_rgba -- --test-threads=1`
  - `cargo test app_core::native_shell::tests::waveform -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Why it matters: the same low-level waveform-image-to-native-RGBA translation exists in two places that straddle the legacy controller and native projection boundary. That is small code, but it is exactly the kind of duplicated contract logic that drifts quietly.
- Evidence:
  - `src/app_core/native_shell/waveform_projection.rs` defines a private `waveform_image_to_native_rgba(...)` helper used by `project_waveform_image(...)`.
  - `src/app/controller/library/wavs/waveform_rendering.rs` defines a second `waveform_image_to_native_rgba(...)` helper with the same conversion logic and exposes it to `src/app/controller/library/wavs/waveform_rendering/reuse.rs`.
  - `docs/ARCHITECTURE.md` explicitly separates host projection/app-core responsibilities from legacy controller paths, but no shared helper or note currently marks one conversion path as authoritative.
- Recommended change: move RGBA translation into one shared helper with one authoritative doc comment, then make both the legacy controller path and the app-core projection path call that helper and add parity tests around it.
- Expected impact: removes a quiet duplication seam on a rendering contract boundary and clarifies ownership without a broad rewrite.
- Risks / tradeoffs: low; the main risk is touching both call sites without preserving the existing fallback behavior noted in `project_waveform_image(...)`.
- Dependencies: none
- Suggested validation:
  - targeted waveform/native-projection tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 5. Add end-to-end controller coverage for loop-crossfade prompt/apply/register/undo flows
- Classification: Test gap
- Confidence: High
- ROI: Medium-High
- Effort: M
- Completed: `2026-03-15`
- Commit: `c39f4352` (`test(playback): cover loop crossfade controller flow`)
- Assumption used: the safe gap to close is controller-side prompt/apply/register/undo coverage, not a change to the existing DSP routine.
- Validation:
  - `cargo test loop_crossfade -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Why it matters: loop crossfade is a user-facing file-producing workflow that touches prompt state, file writes, DB registration, browser refresh, pending playback, and undo. The current tests only cover the low-level DSP helpers.
- Evidence:
  - `src/app/controller/playback/loop_crossfade.rs` is `403` lines.
  - The file owns `request_loop_crossfade_prompt_for_browser_row`, `apply_loop_crossfade_prompt`, `apply_loop_crossfade_for_sample`, `register_loop_crossfade_entry`, and undo-entry construction.
  - The local `#[cfg(test)]` block only tests `find_crossfade_cut_frame(...)` and `apply_loop_crossfade(...)`.
- Recommended change: add focused tests for prompt lifecycle, output naming collisions, DB registration/tag carry-over, pending playback suppression/restart behavior, and undo payload correctness.
- Expected impact: safer edits in a destructive-adjacent workflow and tighter coverage around file/database side effects.
- Risks / tradeoffs: tests will need realistic temp-file/source fixtures, which adds some harness overhead.
- Dependencies: none
- Suggested validation:
  - targeted playback/loop-crossfade tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 6. Thin `src/app/controller/playback/mod.rs` into ownership-based façade modules instead of one mixed gateway
- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Completed: `2026-03-15`
- Commit: `6a5ddf54` (`refactor(playback): split bpm and age helpers`)
- Assumption used: `mod.rs` should remain a façade, but non-façade BPM and playback-age helpers can move beside their owning responsibilities.
- Validation:
  - `cargo test app::controller::playback::tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Why it matters: a large amount of playback behavior has already been split into submodules, but the remaining top-level playback gateway still mixes selection/BPM guard helpers, deferred age-update persistence, playhead helper exposure, and thin browser/tagging/random-navigation façades. That weakens discoverability and keeps the file at the size budget ceiling.
- Evidence:
  - `src/app/controller/playback/mod.rs` is `400` lines.
  - The file still owns `selection_meets_bpm_min(...)`, `bpm_min_selection_seconds(...)`, deferred playback-age commit helpers, playhead helpers, and a long list of façade methods spanning transport, player, tagging, browser navigation, and random navigation.
  - `tmp/cleanup_audit_hotspots.md` still lists the playback module family as oversized debt (`src/app/controller/playback/mod.rs`, `src/app/controller/playback/loop_crossfade.rs`, `src/app/controller/playback/tests.rs`).
- Recommended change: keep `mod.rs` as a narrow export/entry surface and move the remaining non-export helper logic beside the owning transport/player/navigation/tagging modules.
- Expected impact: easier navigation for future playback changes and lower chance of `mod.rs` continuing to accumulate mixed responsibilities.
- Risks / tradeoffs: medium confidence because some façade grouping is intentional; the split should stay conservative and avoid churning public method names without need.
- Dependencies: none
- Suggested validation:
  - targeted playback tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 7. Separate pure folder-selection set logic from controller/UI side effects in `src/app/controller/library/source_folders/selection/ops.rs`
- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Completed: `2026-03-15`
- Commit: `106649d6` (`refactor(folders): extract selection set logic`)
- Assumption used: the existing folder-selection contract is the authority; the refactor should extract pure set/anchor/root-mode rules without broadening or narrowing sibling-selection behavior.
- Validation:
  - `cargo test folders_core -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: folder selection is stateful and affects browser filtering, but the current implementation interleaves path-set algebra, root-mode toggling, focus/anchor updates, row rebuilding, and browser rebuild triggers in one file with repeated controller-side glue.
- Evidence:
  - `src/app/controller/library/source_folders/selection/ops.rs` is `378` lines.
  - The file defines pure helpers like `ancestors`, `remove_descendants`, and `insert_folder`, then mixes them with controller methods such as `replace_folder_selection`, `select_folder_range`, `add_folder_to_selection`, `toggle_folder_row_negation`, and `focus_folder_by_path`.
  - Repeated patterns appear across multiple methods: clear drop target, mutate model, update focused row/scroll target, rebuild folder rows, and conditionally call `rebuild_browser_lists()`.
  - There is no local `#[cfg(test)]` module in the file; surrounding tests live in `src/app/controller/tests/folders_core/selection_filtering.rs` and `src/app/controller/tests/folders_core/focus_rules.rs`.
- Recommended change: extract pure selection-state mutation helpers and root/anchor invariants into a focused module with local tests, leaving controller methods responsible only for wiring UI rebuild and focus side effects.
- Expected impact: easier reasoning about folder-filter correctness and smaller blast radius for future changes to root-only vs descendant selection semantics.
- Risks / tradeoffs: root-row toggling and anchor behavior are subtle; tests must capture current semantics before refactoring.
- Dependencies: none
- Suggested validation:
  - targeted folder selection/focus tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [x] 8. Unify browser folder-filter acceptance semantics across the sync browser pipeline and async search worker
- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: S-M
- Completed: `2026-03-15`
- Commit: `2a229c86` (`refactor(browser): share folder filter semantics`)
- Assumption used: sync and async browser paths should share pure folder-filter hashing and acceptance semantics, but keep separate cache and cancellation models.
- Validation:
  - `cargo test folder_filter -- --test-threads=1`
  - `cargo test browser_search_worker -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Why it matters: the repository now has both a synchronous browser visible-row pipeline and an async search-worker pipeline. Both compute cached folder-filter acceptance maps and their associated cache keys. The duplication is local and reversible, but it is exactly the kind of parity seam that can drift.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs` owns `ensure_folder_acceptance_stage(...)`, `folder_accepts(...)`, and filter/hash logic for sync visible-row construction.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/folders.rs` owns `folder_accepts_for_job(...)`, `build_folder_accepts(...)`, and `folder_filter_hash_for_job(...)` for the async worker path.
  - Both paths ultimately rely on `crate::app::controller::library::source_folders::folder_filter_accepts(...)` and root-mode hashing, but they maintain separate acceptance-cache implementations and hashing helpers.
- Recommended change: keep separate caches if needed for performance, but extract shared pure helpers for folder-filter hashing and acceptance semantics so sync rebuilds and async search results cannot silently diverge.
- Expected impact: lower parity risk between sync and async browser filtering without forcing a large pipeline merge.
- Risks / tradeoffs: medium confidence because the two caches serve different execution models; the safest change is shared pure helpers, not unifying the full pipelines.
- Dependencies: none
- Suggested validation:
  - targeted browser pipeline and search-worker parity tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. What is the intended long-term ownership boundary for native waveform image translation?
- Question: should waveform-image-to-native-RGBA conversion live in the legacy controller path, in `app_core`, or in a small shared helper that both layers depend on?
- Evidence:
  - `src/app_core/native_shell/waveform_projection.rs` and `src/app/controller/library/wavs/waveform_rendering.rs` both implement the conversion.
  - `docs/ARCHITECTURE.md` separates app-core projection from legacy controller behavior but does not call out this contract specifically.
- Why this matters: without an explicit owner, small rendering contract changes can drift across the two paths.
- Affected files/modules:
  - `src/app_core/native_shell/waveform_projection.rs`
  - `src/app/controller/library/wavs/waveform_rendering.rs`
- Risk if guessed incorrectly: a cleanup could reinforce the wrong boundary and make later migration harder.
- Most conservative provisional assumption: centralize only the pure conversion helper now, without moving broader waveform-render ownership across layers.

### [!] 2. How far should native Vello runtime decomposition go before it becomes speculative?
- Question: is the intended direction a few more focused helper modules around the existing runner, or a broader runner-state redesign?
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs`, `runtime_events.rs`, `runtime_input.rs`, and `runtime_render/scene.rs` are already partially split but still center on one large runner-owned state machine.
  - `docs/ARCHITECTURE.md` explains high-level ownership but does not define the preferred internal runtime module boundary.
- Why this matters: the runtime is performance-tuned and timing-sensitive; broad restructuring without a documented boundary risks speculative churn.
- Affected files/modules:
  - `vendor/radiant/src/gui_runtime/native_vello.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_startup.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_render/scene.rs`
- Risk if guessed incorrectly: unnecessary architectural churn in the most latency-sensitive area of the codebase.
- Most conservative provisional assumption: prefer local helper/module extraction that preserves the current runner state and public runtime shape.

### [!] 3. Should sync browser visible-row computation and async search-worker computation converge on one shared semantics layer?
- Question: are the two browser pipelines intentionally separate long-term, or are they transitional split implementations that should share more pure logic?
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs` and `src/app/controller/library/wavs/browser_search_worker/pipeline/*.rs` both compute folder acceptance and filtered/sorted visible rows.
  - The worker pipeline already carries parity-focused tests (`parity_tests`), which suggests semantic equivalence matters.
- Why this matters: without a documented answer, cleanup could either leave too much duplicated logic or over-merge execution models that are intentionally distinct.
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/folders.rs`
- Risk if guessed incorrectly: either recurring parity drift or an over-ambitious merge that harms responsiveness/cancellation behavior.
- Most conservative provisional assumption: share only pure filter/hash semantics first and keep synchronous and asynchronous cache/execution models separate.

### [!] 4. Is `src/app/controller/playback/mod.rs` expected to remain a user-facing façade, or should playback discoverability become module-first?
- Question: should the playback root stay as a convenience façade for most controller entrypoints, or is the preferred direction that callers reach the owning modules directly?
- Evidence:
  - `src/app/controller/playback/mod.rs` already delegates to many focused submodules but still contains a large gateway surface.
  - `docs/ARCHITECTURE.md` documents broad layer ownership, not the preferred internal shape for the playback controller surface.
- Why this matters: the right cleanup is different depending on whether the large façade is intentional API design or just residual debt.
- Affected files/modules:
  - `src/app/controller/playback/mod.rs`
  - `src/app/controller/playback/*`
- Risk if guessed incorrectly: a cleanup could remove an intentionally convenient entrypoint without real net value.
- Most conservative provisional assumption: keep the façade but move non-facade helper logic out of `mod.rs`.

## Rejected Ideas

- `Customizable hotkeys`
  - Why it was considered: the hotkey registry is prominent and user-facing.
  - Why it was rejected: the repository documents deterministic contextual hotkeys, but no roadmap/plan/doc in the current tree suggests user-remappable hotkeys are the intended next direction.
  - Missing evidence: no in-repo product note, config surface, or issue-plan pointer for remapping support.

- `Promote desktop AIV smoke into CI now`
  - Why it was considered: GUI coverage is an active system and CI promotion is a common follow-up.
  - Why it was rejected: `docs/gui_test_platform.md` still explicitly marks desktop AIV as local-only because Windows foreground/focus stability remains a blocker.
  - Missing evidence: no fresh doc or config change showing repeated stability evidence or approval for CI promotion.

- `Broad crate split of the native Vello runtime`
  - Why it was considered: `native_vello` still has several large runtime files.
  - Why it was rejected: the repository provides strong evidence for local decomposition needs, but not for a cross-crate split being the highest-value next step.
  - Missing evidence: no active architecture note, roadmap, or validation bottleneck pointing to crate boundaries as the present constraint.

## Execution Notes

- Blocked items: none
- Clarification-needed items during execution: none
- Rejected items added during execution: none
- Broadest validation status:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed
