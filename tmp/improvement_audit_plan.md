# Evidence-Driven Improvement Audit Plan

- Generated (UTC): `2026-03-15T12:42:32Z`
- Repository: `C:\dev\sempal`
- Branch / head: `next` / `20f83666`
- Phase: `Phase 2 in progress`
- Status: `Sequential backlog implementation is in progress on 2026-03-15; items 1-4 are complete and items 5-9 remain pending.`
- Validation baseline:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` regenerated `tmp/cleanup_audit_hotspots.md` from the current branch head.

## Repository Context

- Project purpose: realtime-oriented desktop sample triage and editing tool focused on responsive auditioning, discovery, and curation.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`
- Maturity level: active, guardrail-heavy application with a large Rust workspace, a custom GUI runtime in `vendor/radiant`, and multiple maintained validation loops.
  - Confidence: Strongly implied by code/docs
  - Evidence: `Cargo.toml`, `docs/README.md`, `.github/workflows/ci.yml`, `docs/QUALITY_SCORE.md`
- Primary languages / frameworks / tooling: Rust 2024 workspace, Cargo, PowerShell-first Windows wrappers, `vendor/radiant` for GUI/runtime behavior, and `cargo-nextest` in CI.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `README.md`, `.github/workflows/ci.yml`, `docs/TEST.md`
- Repository shape: root app logic in `src/`, companion apps in `apps/`, support tools in `tools/`, developer docs in `docs/`, live planning state in `tmp/`, and GUI/runtime code in `vendor/radiant/`.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `docs/ARCHITECTURE.md`, `docs/README.md`
- Architectural boundaries: `src/**` owns domain logic and UI intent, `src/app_core/**` owns backend-neutral action/projection contracts, and `vendor/radiant/**` owns GUI behavior, layout, input routing, and runtime rendering.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`
- Test strategy: fast PowerShell wrappers for the inner loop, `ci_local` for broader parity, semantic GUI contract/suite loops, and targeted `radiant` native-shell/runtime tests beside the main Rust unit/integration suite.
  - Confidence: Explicitly documented
  - Evidence: `docs/TEST.md`, `docs/gui_test_platform.md`, `scripts/ci_quick.ps1`, `scripts/run_gui_contract.ps1`, `scripts/run_gui_suite.ps1`
- Canonical local validation on Windows:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `AGENTS.md`, `docs/README.md`, `docs/TEST.md`
- Documented priorities: non-blocking interaction, deterministic contextual hotkeys, clear state ownership, responsive GUI/runtime behavior, and trustworthy reversible sample-management flows.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`, `docs/ARCHITECTURE.md`
- Explicit non-goals: DAW replacement, cloud/social platform behavior, attention-capture features, and speculative broad redesigns without evidence.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`

## ROI-Ranked Backlog

### 1. [x] Decompose the browser search-worker stage hub into cache refresh, scoring, and visible-row builders

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Browser search remains a responsiveness-critical path, but the worker-side pipeline still centralizes DB reopen logic, compact-entry reloads, query-score cache reuse, fast-path short-circuiting, filter/folder gating, similarity handling, and sort application in one file. The current shape raises the cost of changing search behavior safely.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` at 456 lines and flags it as a likely test-gap hotspot.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` defines `ensure_search_cache_ready_for_job`, `ensure_search_entries_loaded_for_job`, `resolve_query_scores_for_job`, `build_fast_path_result_if_applicable`, `build_visible_rows_for_job`, and `build_visible_rows_for_similar` in one module.
  - `docs/design_principles.md` treats responsiveness as a correctness issue and requires long-running work to remain non-blocking and observable.
- Recommended change:
  - Split the file into focused modules for source/DB cache refresh, query-score cache and scoring, and visible-row construction/similarity sorting.
  - Add direct stage-level tests for cache reuse, cancellation boundaries, and similarity-sort anchoring so worker behavior is not characterized only indirectly.
- Expected impact:
  - Reduces regression risk in a hot async pipeline.
  - Makes future browser-search performance work easier to reason about and measure.
- Risks / tradeoffs:
  - Search ordering and cancellation semantics are user-visible and performance-sensitive.
  - A careless split could accidentally perturb the worker fast path or score-cache reuse rules.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted `browser_search_worker` tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Commit: `01fc050b`
- Assumptions used:
  - The split remains a file-boundary change only; worker ordering, cancellation, and score-cache semantics stay unchanged.
  - `pipeline.rs` and existing worker tests should continue to rely on the `stages` facade instead of importing child modules directly.
- Validation outcome:
  - `cargo fmt --all`
  - `cargo test browser_search_worker -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Plan-order deviation: None

### 2. [x] Split the analysis-job progress poller into source discovery, aggregation, stale cleanup, and loop orchestration

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Analysis progress is part of the app’s non-blocking trust surface, but the poller still mixes condvar wakeup state, source enumeration, cached total aggregation, stale-job cleanup, DB refresh policy, and the background thread loop in one concurrency-sensitive file.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/analysis_jobs/pool/job_progress.rs` at 418 lines.
  - `src/app/controller/library/analysis_jobs/pool/job_progress.rs` defines `ProgressPollerWakeup`, `refresh_sources`, `current_progress_all`, `cleanup_stale_jobs`, `spawn_progress_poller`, and `should_refresh_db` together.
  - `docs/design_principles.md` requires long-running work to be observable and non-blocking.
- Recommended change:
  - Extract source discovery/refresh, cached aggregation, and stale cleanup into focused helpers or modules.
  - Leave `spawn_progress_poller` as a thin orchestration loop with explicit timing policy.
  - Keep the existing cache/message behavior intact and add direct tests for refresh cadence and touched-source cache updates where useful.
- Expected impact:
  - Lowers concurrency-regression risk in progress reporting and stale-job cleanup.
  - Makes future observability and worker-pool changes easier to review.
- Risks / tradeoffs:
  - Poll timing, wakeup behavior, and cache refresh policy are timing-sensitive.
  - Any ownership split must preserve current repaint and message emission semantics.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted `job_progress` tests
  - Analysis-job pool tests covering cleanup/progress
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Commit: `96a50782`
- Assumptions used:
  - The split preserves the existing wakeup, repaint, and message-emission behavior of the progress poller loop.
  - The cached aggregate remains the only reusable source of truth between DB refreshes; no new durable state is introduced.
- Validation outcome:
  - `cargo fmt --all`
  - `cargo test job_progress -- --test-threads=1`
  - `cargo test analysis_jobs::pool -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Plan-order deviation: None

### 3. [x] Break the hotkey registry into scope-focused tables and keep one explicit export surface

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Contextual hotkeys are a first-class interaction contract, but one 516-line file still acts as the single edit point for every gesture across global, browser, folder, and waveform scopes. Even with direct invariant tests now in place, that hub is still expensive to change safely and remains on the file-size allowlist.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/ui/hotkeys/actions.rs` at 516 lines.
  - `docs/file_size_budget_allowlist.txt` still allowlists `src/app/controller/ui/hotkeys/actions.rs`.
  - `src/app/controller/ui/hotkeys/actions.rs` is almost entirely one `HOTKEY_ACTIONS` table plus tests.
  - `src/app/controller/ui/hotkeys/types.rs` shows the real domain split is already scope/gesture/command based.
  - `docs/design_principles.md` says hotkeys must be contextual, explicit, predictable, and inspectable.
- Recommended change:
  - Split the static registry into scope-focused declarative chunks, keeping one exported combined registry for lookup behavior.
  - Keep the current invariant tests and add any missing table-shape tests at the combined export boundary rather than inside one giant table file.
- Expected impact:
  - Reduces review noise and keymap drift risk for a user-facing control surface.
  - Removes one of the largest remaining allowlisted controller files.
- Risks / tradeoffs:
  - Registry order, ids, and gestures are compatibility surfaces.
  - Over-abstracting a mostly declarative table would make it harder to audit, not easier.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo test hotkey_registry -- --test-threads=1`
  - `cargo test hotkey_helper_views -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Commit: `37e69684`
- Assumptions used:
  - Registry export order remains the compatibility surface, even while the actual action definitions move into scope-owned files.
  - The file-size budget allowlist should drop `src/app/controller/ui/hotkeys/actions.rs` once the registry hub is split below the limit.
- Validation outcome:
  - `cargo fmt --all`
  - `cargo test hotkey_registry -- --test-threads=1`
  - `cargo test hotkey_helper_views -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Plan-order deviation: None

### 4. [x] Split browser-controller row actions by mutation family instead of keeping one omnibus action hub

- Classification: Refactor / cleanup
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The browser controller still centralizes tagging, loop-marker changes, BPM writes, normalization, loop-crossfade, rename, delete, and dead-link removal behind one trait impl. That mixes multiple persistence and focus-restoration policies in a single mutation hub.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/browser_controller/actions.rs` at 402 lines and flags it as a likely test-gap hotspot.
  - `src/app/controller/library/browser_controller/actions.rs` defines the `BrowserActions` trait with multiple unrelated command families and shares resolution/focus logic like `resolve_unique_browser_contexts`, `next_browser_focus_after_delete`, and `refocus_after_filtered_removal`.
  - The same file handles source-grouped BPM updates, playback restoration after loop-crossfade, and delete/dead-link focus recovery.
- Recommended change:
  - Split the file by mutation family, for example metadata/tagging, destructive/removal flows, and audio-transform flows.
  - Keep shared row-context resolution and refocus helpers in a small common helper layer.
- Expected impact:
  - Reduces blast radius for user-visible browser mutation changes.
  - Makes it easier to add targeted tests for one action family without touching unrelated flows.
- Risks / tradeoffs:
  - Delete/focus restoration and playback-retarget behavior must stay stable.
  - Multi-row grouping logic is shared and easy to duplicate accidentally.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted browser-controller action tests
  - Undo/redo coverage for affected flows
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completed: `2026-03-15`
- Commit: `pending`
- Assumptions used:
  - The existing browser-action regression tests adequately cover the current focus-recovery and multi-row mutation contracts for this file-boundary split.
  - The shared row-resolution and post-delete focus logic remain internal helpers, not separate public APIs.
- Validation outcome:
  - `cargo fmt --all`
  - `cargo test browser_actions -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Plan-order deviation: None

### 5. [ ] Separate source selection, source lifecycle/remap, and missing/watcher maintenance in `sources.rs`

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters:
  - Source management is foundational for the library model, but one controller file still mixes source selection, add/remove/remap flows, missing-source repair, watcher refresh, UI row projection, and config persistence. That makes source-lifecycle changes riskier than they need to be.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/library/sources.rs` at 422 lines.
  - `src/app/controller/library/sources.rs` owns `select_source_internal`, `add_source_from_path`, `remove_source`, `refresh_sources_ui`, `refresh_source_watcher`, `rebuild_missing_sources`, `mark_source_missing`, `remap_source_to`, and `open_source_folder`.
  - `docs/design_principles.md` emphasizes data integrity and trust for library mutations.
- Recommended change:
  - Split source selection/loading orchestration from source lifecycle/remap operations and from missing/watcher maintenance.
  - Make the remap/remove path explicit about which caches, UI rows, and persisted selections are invalidated.
- Expected impact:
  - Lowers regression risk in source-root changes and missing-source recovery.
  - Makes future folder/source features easier to implement without touching one large hub.
- Risks / tradeoffs:
  - Source-id preservation, DB copy/remap behavior, and selected-source recovery are trust-sensitive.
  - A split that obscures cache invalidation order would be a regression.
- Dependencies:
  - None.
- Suggested validation:
  - Source add/remove/remap tests
  - Missing-source recovery tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Separate pure option-setting policy from live playback/waveform side effects in `interaction_options.rs`

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - Interaction options are user-facing and stateful, but the current controller file mixes clamp/normalization logic, config persistence, input-monitor restarts, anti-clip player updates, waveform reloads, and replay-after-BPM-change behavior in one setter-heavy hub.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/ui/interaction_options.rs` at 428 lines.
  - `src/app/controller/ui/interaction_options.rs` owns pure clamp helpers at the top, then a long `impl AppController` with setters that trigger audio-player mutation, waveform reload, replay, and config persistence.
  - `docs/design_principles.md` prioritizes predictability and immediate auditioning.
- Recommended change:
  - Extract pure control normalization and state-mutation policy from the live side-effect paths.
  - Keep explicit controller entrypoints, but move replay/reload/player-monitor behavior into focused helpers that make their side effects obvious.
- Expected impact:
  - Makes option changes safer to review and test directly.
  - Reduces the chance that a new persisted control unintentionally inherits the wrong live side effects.
- Risks / tradeoffs:
  - BPM/stretch and monitoring settings have live audio consequences.
  - The split should not hide when persistence is expected versus when preview-only behavior is intended.
- Dependencies:
  - None.
- Suggested validation:
  - Interaction-options tests
  - Playback option/reload tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 7. [ ] Break `SampleBrowserState` into focused selection, viewport, and search/similarity state slices

- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The browser’s durable UI state is still concentrated in one 439-line struct that mixes triage partitions, visible-row revisions and reverse lookups, multi-selection caches, marker cache state, search state, similarity state, prompts, and copy-flash state. That centrality makes safe browser work harder even after the earlier selection-authority cleanup.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/state/browser.rs` at 439 lines and `src/app/controller/library/wavs/browser_actions/selection.rs` at 356 lines.
  - `src/app/state/browser.rs` defines one `SampleBrowserState` with selection, viewport, reverse-lookup caches, search/similarity, prompts, and visual flash state together.
  - `src/app/controller/library/wavs/browser_pipeline.rs` and `src/app/controller/library/wavs/browser_actions/selection.rs` still depend on overlapping pieces of that state.
- Recommended change:
  - Introduce focused nested structs or modules for browser selection/markers, visible-row/viewport bookkeeping, and query/similarity state.
  - Preserve the current path-authoritative selection behavior and keep derived caches explicit.
- Expected impact:
  - Makes the remaining browser code easier to audit and change without broad state churn.
  - Gives clearer ownership boundaries for future browser refactors.
- Risks / tradeoffs:
  - Browser selection, lookup caches, and busy/request ids are tightly coupled.
  - If the new split is too granular, it could just move complexity around without clarifying ownership.
- Dependencies:
  - None.
- Suggested validation:
  - Browser selection/marker tests
  - Browser pipeline tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 8. [ ] Expand GUI scenario and desktop-AIV packs for transport, volume drag, and map-point interaction

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The semantic GUI platform is already part of the quick loop, but the current scenario packs still stop short of several user-visible interaction surfaces that the docs explicitly call out as the next coverage slice.
- Evidence:
  - `docs/gui_test_platform.md` lists “Add more seeded fixtures and scenario assertions for transport, volume drag, and map-point interaction” as the first immediate next step.
  - `src/gui_test/packs.rs` currently ships only the `contract-smoke` pack with scenarios for browser search, waveform seek/zoom, options, prompt, and update panel flows.
  - `src/gui_test/aiv/packs.rs` desktop regression cases cover browser, prompt, update, and waveform transport/selection, but not dedicated transport-button, volume-drag, or map-point semantic cases.
- Recommended change:
  - Add deterministic fixture seeds and scenario assertions for the missing transport, volume-drag, and map-point interactions in both the in-process GUI pack and the desktop-AIV manifest where appropriate.
  - Keep coverage tied to stable semantic ids instead of screenshot-only assertions.
- Expected impact:
  - Improves the value of the already-promoted semantic GUI contract lane.
  - Reduces reliance on ad hoc manual verification for important micro-interactions.
- Risks / tradeoffs:
  - New semantic ids and assertions become compatibility surfaces for GUI tooling.
  - Poor fixture seeding could make GUI coverage noisy instead of useful.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 9. [ ] Harden the desktop-AIV smoke wrapper around foreground/focus failures before considering broader promotion

- Classification: Developer-experience improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The repository already ships a usable desktop-AIV wrapper, but the docs still call out Windows foreground/focus behavior as the reason it is not safe to promote into CI. That limits confidence in the end-to-end desktop loop the project is already investing in.
- Evidence:
  - `docs/gui_test_platform.md` says desktop AIV stability still depends on Windows foreground/focus behavior and lists hardening the AIV smoke wrapper as an immediate next step.
  - `scripts/run_gui_aiv_smoke.ps1` is only a thin wrapper over `run_gui_aiv_suite.ps1`.
  - `scripts/run_gui_aiv_suite.ps1` currently does a straightforward `Wait-ForWindow`, one `Ensure-WindowForeground`, then step execution; on failure it captures screenshots but does not encode a stronger retry/reacquire policy in this top-level flow.
- Recommended change:
  - Add explicit foreground-retry and re-acquire handling at the desktop wrapper level, plus clearer failure categorization for focus-related flake versus application-level failures.
  - Keep the contract loop in `ci_quick`; do not promote desktop AIV gating until the wrapper has objective stability evidence.
- Expected impact:
  - Makes the desktop-AIV loop more trustworthy for local regression work.
  - Produces cleaner diagnostics when Windows focus behavior is the actual failure mode.
- Risks / tradeoffs:
  - Desktop automation can still fail for environment reasons outside the app’s control.
  - Over-retrying could hide true regressions if failure classification is too loose.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_smoke.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_suite.ps1 -PackName desktop-regression`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What stability bar should desktop AIV meet before it is eligible for CI or any stronger default gate?

- Evidence:
  - `docs/gui_test_platform.md` says desktop AIV is useful locally but “not yet stable enough to promote into CI.”
  - The same doc lists Windows foreground/focus behavior as the blocking limitation.
- Why this matters:
  - Without a defined promotion bar, work on the AIV wrapper can drift between “local convenience” and “intended CI contract” without clear success criteria.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `scripts/run_gui_aiv_smoke.ps1`
  - `scripts/run_gui_aiv_suite.ps1`
  - `src/gui_test/aiv/packs.rs`
- Risk if guessed incorrectly:
  - The team could either under-invest in a strategic test loop or accidentally turn a flaky desktop loop into a noisy gate.
- Most conservative provisional assumption:
  - Keep desktop AIV local-only and non-mandatory until repeated local evidence shows stable foreground/focus handling.

### [!] What is the intended long-term ownership boundary between durable browser UI state and derived browser pipeline caches?

- Evidence:
  - `src/app/state/browser.rs` still stores durable browser selection, visible-row revisions, reverse-lookup caches, search state, and similarity state together.
  - `src/app/controller/library/wavs/browser_pipeline.rs` keeps separate retained pipeline caches in `ui_cache.browser.pipeline`.
  - `src/app/controller/library/wavs/browser_actions/selection.rs` still owns large selection/anchor behavior helpers around the same browser state.
- Why this matters:
  - Safe future browser refactors depend on knowing which data is meant to be durable state versus derivable cache or controller helper state.
- Affected files/modules:
  - `src/app/state/browser.rs`
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_actions/selection.rs`
- Risk if guessed incorrectly:
  - Refactors could either duplicate cache state again or over-couple controller helpers to data that should be ephemeral.
- Most conservative provisional assumption:
  - Keep durable user-visible browser state in `SampleBrowserState`, but move derivable caches and algorithmic helpers behind smaller focused wrappers rather than inventing new durable state.

### [!] What exact remap/remove contract should source management preserve for DB history, cache invalidation, and focused selection recovery?

- Evidence:
  - `src/app/controller/library/sources.rs` copies `.sempal_samples.db` during remap when the destination DB is absent, preserves source ids, clears caches, and conditionally clears waveform/browser state in one flow.
  - There is no single dedicated source-lifecycle doc that defines the intended user-visible guarantees for remap/remove behavior.
- Why this matters:
  - Source lifecycle work touches trust-sensitive library state, and the safest refactor boundary depends on which of these behaviors are strict contract versus implementation detail.
- Affected files/modules:
  - `src/app/controller/library/sources.rs`
  - `src/sample_sources/library.rs`
  - `src/sample_sources/db/*`
- Risk if guessed incorrectly:
  - A cleanup refactor could silently change tag/history preservation or selected-source recovery behavior.
- Most conservative provisional assumption:
  - Preserve source ids, reuse existing DB history when safe, and keep current selection/waveform clearing behavior unless a dedicated source-lifecycle contract is documented.

## Rejected Ideas

### [-] Promote desktop AIV smoke into `ci_quick` or CI now

- Why it was considered:
  - The repo already has semantic GUI artifacts, desktop manifests, and wrapper scripts.
- Why it was rejected:
  - Current docs explicitly say the desktop-AIV loop is not yet stable enough because of Windows foreground/focus behavior.
- What evidence was missing:
  - No current stability threshold or repeated green desktop evidence that would justify promotion.

### [-] Add configurable hotkeys

- Why it was considered:
  - The hotkey registry is large and highly visible.
- Why it was rejected:
  - A large registry is not evidence that user-customizable keymaps are an intended product direction.
- What evidence was missing:
  - No roadmap note, doc section, or existing config surface suggests customizable hotkeys are planned.

### [-] Remove the legacy runtime compatibility path entirely

- Why it was considered:
  - `README.md` emphasizes the `radiant` path as the active default runtime.
- Why it was rejected:
  - The architecture docs still describe `src/legacy_runtime` as a compatibility path rather than dead code.
- What evidence was missing:
  - No doc or active plan says the legacy runtime should now be deleted outright.

### [-] Start another broad crate-split or build-system redesign lane

- Why it was considered:
  - The workspace is large and performance/build docs exist.
- Why it was rejected:
  - The active documentation already tracks a completed runtime-performance lane and does not present a current build-architecture crisis in the refreshed tree.
- What evidence was missing:
  - No fresh timing artifact or active doc says another structural crate split is currently the highest-value next step.
