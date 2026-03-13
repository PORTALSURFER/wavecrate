# Evidence-Driven Improvement Audit Plan

Generated: 2026-03-13
Repository: `C:\dev\sempal`
Branch: `next`

## Repository Context

- Project purpose: Rust desktop audio triage tool with a native GUI, waveform
  editing/playback, source management, update flow, and GUI automation/test
  harnesses.
  - Basis: Explicitly documented in `README.md`, `docs/ARCHITECTURE.md`,
    `docs/TEST.md`.
- Maturity level: active, heavily iterated application with substantial test
  and CI guardrails, but still carrying several large native-GUI and helper
  modules.
  - Basis: Strongly implied by `.github/workflows/ci.yml`,
    `docs/QUALITY_SCORE.md`, `docs/plans/active/runtime_performance_exec_plan.md`,
    and the current hotspot scan.
- Primary languages/tooling: Rust workspace with Cargo, PowerShell wrappers on
  Windows, native GUI/runtime code in `vendor/radiant`, GUI automation through
  semantic snapshots plus AIV desktop runs.
  - Basis: Explicitly documented in `AGENTS.md`, `Cargo.toml`, `docs/TEST.md`,
    `docs/gui_test_platform.md`.
- Architectural boundaries: app/controller logic in `src/app/**` and
  `src/app_core/**`, native GUI/runtime implementation in `vendor/radiant`,
  updater UI in `apps/updater-helper`, test/automation support in
  `src/gui_test/**`.
  - Basis: Strongly implied by code layout and `docs/ARCHITECTURE.md`.
- Test strategy: deterministic Rust unit/integration tests first, then GUI
  contract/AIV coverage for native workflows.
  - Basis: Explicitly documented in `docs/TEST.md`,
    `docs/gui_test_platform.md`.
- Canonical Windows validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Basis: Explicitly documented in `AGENTS.md`, `README.md`, `docs/TEST.md`.
- Documented priorities: correctness, maintainability, deterministic tests,
  PowerShell-first Windows workflows, and semantic-first GUI automation.
  - Basis: Explicitly documented in `AGENTS.md`,
    `docs/gui_test_platform.md`, `docs/TEST.md`.
- Explicit non-goals found:
  - No evidence for large product redesigns.
  - No evidence for replacing semantic GUI testing with pixel-only testing.
  - No evidence for broad framework swaps.

## ROI-Ranked Backlog

### [x] 1. Split native-shell automation snapshot building by surface and shared helper layer

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The native automation snapshot is the semantic contract for GUI tests, AIV,
    and tooling. Keeping all surface builders in one file makes regressions in
    node ids, metadata, and action wiring harder to review and harder to test
    locally.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/automation.rs` is 1088 lines in
    the current tree.
  - The file currently owns root snapshot assembly plus top bar, sidebar,
    waveform, browser, status bar, options, prompt, and progress automation.
  - `docs/gui_test_platform.md` and `src/gui_test/aiv/mod.rs` depend on stable
    semantic ids/actions emitted from this path.
- Recommended change:
  - Split `automation.rs` into focused modules by panel family
    (`top_bar`, `sidebar`, `waveform`, `browser`, `dialogs`) plus a shared
    helper module for node ids, bounds, metadata, and action slug conversion.
  - Add direct contract tests around shared helper behavior and one module-level
    smoke test per surface.
- Expected impact:
  - Lower regression risk in GUI automation coverage.
  - Better locality when adding/changing automation nodes.
  - Faster review of semantic-contract changes.
- Risks / tradeoffs:
  - Moderate churn in a high-touch file.
  - Must preserve existing stable node ids and action ids.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted `radiant` automation snapshot tests.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `ee5b868e` (`refactor(gui): split automation snapshot surfaces`)
  - Assumption used: stable automation node ids and action ids remain unchanged while only module ownership shifts.
  - Validation: `cargo test -p radiant automation::tests -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 2. Split browser-row layout/windowing helpers out of the current multi-purpose cache file

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Current browser-row behavior has been a repeated regression hotspot. The
    main helper file still mixes row caching, viewport math, truncation,
    scrollbar geometry, toolbar layout, hover colors, source/sidebar geometry,
    and volume-meter helpers, which makes future scroll/focus fixes harder to
    reason about.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/browser_rows.rs` is 804 lines.
  - It defines unrelated concerns including `SidebarRowsCacheKey`,
    `BrowserRowsCacheKey`, `BrowserToolbarLayout`, scrollbar math, truncation,
    color helpers, and row-window calculations such as
    `browser_rows_window_bounds_with_previous` and
    `browser_window_start_with_previous`.
  - Recent browser autoscroll/focus regressions have landed repeatedly in this
    area, which strongly implies ongoing churn.
- Recommended change:
  - Split into focused modules for:
    - browser viewport/windowing math,
    - browser text truncation,
    - browser toolbar/scrollbar geometry,
    - shared row color/fill helpers,
    - sidebar/source row geometry.
  - Keep public helper entry points small and purpose-specific.
- Expected impact:
  - Lower chance of reintroducing autoscroll/focus bugs.
  - Better discoverability for row math versus visual styling helpers.
- Risks / tradeoffs:
  - Moderate refactor risk in native GUI layout code.
  - Needs careful preservation of current viewport/guard-band semantics.
- Dependencies:
  - None.
- Suggested validation:
  - Existing browser-row native tests.
  - Browser AIV regression pack.
  - `devcheck.ps1` and `ci_quick.ps1`.
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `943f908b` (`refactor(gui): split browser row helpers`)
  - Assumption used: no behavior changes are intended; the split preserves the current autoscroll, truncation, and row-hit semantics while reducing mixed-responsibility coupling.
  - Validation: `cargo test -p radiant browser_rows -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [ ] 3. Remove `missing_docs` suppressions from the public GUI contract surfaces

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - These enums and scenario/assertion types are contract surfaces consumed by
    host tests, tooling, GUI snapshots, and AIV manifests. Suppressing docs on
    them contradicts the repository documentation rules and makes behavior less
    discoverable for future maintainers.
- Evidence:
  - `src/app_core/actions/catalog.rs` has `#[allow(missing_docs)]` on
    `GuiActionKind`, `GuiSurface`, `GuiEffectClass`, and `GuiCoverageLayer`.
  - `src/gui_test/scenario.rs` has `#[allow(missing_docs)]` on
    `GuiScenarioStep` and `GuiAssertion`.
  - `AGENTS.md` explicitly requires clear doc comments for public-facing
    objects.
- Recommended change:
  - Replace suppressions with concise, behavior-focused docs for each enum and
    each non-obvious variant.
  - If any public names are unclear while documenting them, rename the ones with
    poor domain clarity before the docs are written.
- Expected impact:
  - Better contract readability.
  - Less ambiguity when extending automation or action coverage.
- Risks / tradeoffs:
  - Low technical risk.
  - Some variants may expose naming problems that require small follow-up
    renames.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo doc` via existing CI flow.
  - `devcheck.ps1` and `ci_quick.ps1`.
- Product clarification required: No

### [ ] 4. Add direct helper coverage and smaller internal seams for waveform line rasterization

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The waveform line renderer is dense, math-heavy, and performance-sensitive.
    It currently concentrates interpolation, supersampling, alpha blending, and
    anti-aliased line stepping in one file, which increases the chance of subtle
    rendering regressions without narrow tests.
- Evidence:
  - `src/waveform/render/paint/lines.rs` is 503 lines.
  - It owns `sample_at_frame`, `supersampled_frame`, `catmull_rom`,
    `blend_pixel`, and `draw_line_aa`.
  - The file exposes internal helper seams but does not keep the helper logic in
    separate focused modules with obvious direct tests nearby.
- Recommended change:
  - Split raster math into smaller helper units
    (sample interpolation, supersampling, AA stepping, pixel blending).
  - Add direct deterministic tests for boundary cases:
    - zero-width/zero-height images,
    - steep versus vertical lines,
    - channel clamping,
    - end-of-buffer interpolation,
    - alpha blend edge cases.
- Expected impact:
  - Better confidence in a sensitive render path.
  - Faster root-cause analysis when waveform visuals regress.
- Risks / tradeoffs:
  - Moderate effort because helper extraction must not hurt hot-path clarity.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted waveform render tests.
  - Existing CI wrappers.
- Product clarification required: No

### [ ] 5. Continue decomposing the native Vello runtime around lifecycle, input, and render ownership

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: L
- Why it matters:
  - The native runtime still keeps a very large runner struct and many
    cross-cutting fields in one root file. This makes state transitions, input
    routing, repaint behavior, and startup/render sequencing harder to audit for
    correctness.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello.rs` is 752 lines even after
    multiple submodules exist.
  - `NativeVelloRunner` still owns startup sequencing, repaint scheduling,
    render caches, pointer/drag state, text input state, clipboard state,
    profiling, and window/surface/render objects in one top-level file.
  - `docs/plans/active/runtime_performance_exec_plan.md` shows this runtime is a
    long-running performance and responsiveness hotspot.
- Recommended change:
  - Extract cohesive state holders and helpers for:
    - startup/window reveal sequencing,
    - transient input/drag state,
    - render cache/state management,
    - text input/clipboard ownership.
  - Keep the root runner focused on orchestration instead of raw state storage.
- Expected impact:
  - Better reasoning about lifecycle bugs and repaint/input races.
  - Lower cost for future runtime fixes.
- Risks / tradeoffs:
  - Highest refactor risk in this backlog.
  - Must preserve current performance wins and event ordering.
- Dependencies:
  - None, but should follow after smaller high-confidence items if the lane needs
    to de-risk first.
- Suggested validation:
  - Existing `radiant` runtime tests.
  - GUI contract/AIV suites.
  - `ci_quick.ps1`, and likely `ci_local.ps1` before push.
- Product clarification required: No

### [ ] 6. Split updater-helper UI orchestration into background tasks, state transitions, and view projection

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The updater companion UI currently mixes background threads, release list
    loading, selection movement, log buffering, update execution, and panel
    projection in one file. That raises the chance of stale state and
    order-of-operations mistakes in a user-facing installer/update path.
- Evidence:
  - `apps/updater-helper/src/ui.rs` is 517 lines.
  - `UpdateNativeBridge` owns background receivers, release polling, log
    management, selection movement, update start, and panel model construction.
  - `run_gui`, `refresh_release_list`, `poll_background_updates`, and
    `update_panel_model` all live together.
- Recommended change:
  - Split into:
    - async/background task helpers,
    - UI state/reducer transitions,
    - view-model projection helpers.
  - Add focused tests for reducer-like state transitions where practical.
- Expected impact:
  - Safer update-flow maintenance.
  - Better readability around background polling and status transitions.
- Risks / tradeoffs:
  - Moderate refactor effort.
  - Companion app tests may need new scaffolding if they do not already exist.
- Dependencies:
  - None.
- Suggested validation:
  - Updater-helper targeted tests if present or added.
  - Existing repo CI wrappers.
- Product clarification required: No

### [ ] 7. Split compute-worker batch execution from queue loop and deferred-finalization policy

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The compute worker handles queue polling, cancellation/shutdown, settings
    capture, decoded-batch grouping, panic wrapping, DB connection management,
    immediate job finalization, deferred updates, and repaint signaling in one
    path. That is a lot of correctness-sensitive behavior in one worker loop.
- Evidence:
  - `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs`
    is a large worker file starting with `spawn_compute_worker`,
    `run_compute_worker`, `process_batch`, `process_batch_work`, and
    `run_work_item`.
  - The file directly owns `connections`, `deferred_updates`, panic recovery,
    queue logging, and post-batch flushing.
- Recommended change:
  - Split into:
    - worker loop/orchestration,
    - work-item execution,
    - deferred update flushing/finalization,
    - batch settings/context collection.
  - Add targeted tests for panic recovery and deferred flush behavior if missing.
- Expected impact:
  - Lower risk when changing analysis job execution.
  - Better auditability around cancellation and deferred DB updates.
- Risks / tradeoffs:
  - Moderate refactor in threaded code.
  - Needs careful preservation of queue throughput and repaint cadence.
- Dependencies:
  - None.
- Suggested validation:
  - Existing analysis-job tests.
  - `devcheck.ps1` and `ci_quick.ps1`.
- Product clarification required: No

### [ ] 8. Separate waveform zoom-cache core behavior from telemetry bookkeeping

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The cache logic is currently entangled with a large set of global telemetry
    counters and emission helpers. That makes core cache behavior harder to
    read, and it increases the cost of changing eviction or locking behavior.
- Evidence:
  - `src/waveform/zoom_cache.rs` is 489 lines.
  - The file combines `WaveformZoomCache` and `CacheInner` behavior with many
    global `AtomicU64` counters, `OnceLock`, telemetry emission helpers, and
    resident-byte accounting functions.
- Recommended change:
  - Move telemetry counters/emission into a dedicated helper module or type.
  - Keep the cache file focused on keying, lookup, insertion, eviction, and
    poison recovery.
- Expected impact:
  - Clearer cache ownership and easier future changes.
  - Better separation between behavior and observability.
- Risks / tradeoffs:
  - Moderate churn in a performance-sensitive path.
  - Must keep current telemetry semantics intact.
- Dependencies:
  - None.
- Suggested validation:
  - Existing waveform cache tests and perf-sensitive guardrails.
  - `ci_quick.ps1`.
- Product clarification required: No

### [ ] 9. Separate pure audio-option normalization from controller-side probing and mutation

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The file already shows a clean pure normalization seam, but the controller
    still mixes probe policy, warning construction, settings mutation, UI view
    projection, persistence, and player rebuild triggers in the same unit. That
    makes audio settings behavior harder to test in isolation.
- Evidence:
  - `src/app/controller/playback/audio_options.rs` contains
    `normalize_audio_options` plus `refresh_audio_options`,
    `refresh_audio_input_options`, multiple setter methods,
    `apply_audio_selection`, and status/player rebuild helpers.
  - Both output and input refresh paths duplicate host/device/sample-rate view
    projection patterns.
- Recommended change:
  - Keep pure normalization/probe-policy helpers separate from controller
    mutation methods.
  - Introduce small shared helpers for output/input view projection and warning
    handling.
- Expected impact:
  - Easier targeted testing of audio settings behavior.
  - Lower coupling between probe policy and UI/controller mutation.
- Risks / tradeoffs:
  - Moderate refactor touching user settings and player rebuild flows.
- Dependencies:
  - None.
- Suggested validation:
  - Existing audio option tests plus full CI wrappers.
- Product clarification required: No

### [ ] 10. Split oversized test and fixture catalogs into domain-focused modules

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - Large test hubs and case catalogs reduce discoverability and make regression
    coverage harder to extend without merge pressure.
- Evidence:
  - `src/gui_test/aiv/packs/cases.rs` is 555 lines and contains many unrelated
    desktop GUI cases.
  - `src/app_core/native_bridge/tests/projection_cache.rs` is 609 lines.
  - `src/app_core/controller/tests.rs` is 531 lines.
  - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs` is 860
    lines.
- Recommended change:
  - Split these hubs by domain or behavior family instead of continuing to grow
    them as single files.
  - Keep module names aligned with the behavior under test, not generic
    “miscellaneous” buckets.
- Expected impact:
  - Easier navigation and lower review friction for new regressions/tests.
- Risks / tradeoffs:
  - Medium confidence because this is more about maintainability than immediate
    correctness.
  - Should avoid noisy test-only churn without behavior gain.
- Dependencies:
  - None.
- Suggested validation:
  - Existing test suite and CI wrappers.
- Product clarification required: No

### [ ] 11. Add Windows parity for the cleanup-hotspot audit helper or document its Bash-only status precisely

- Classification: Developer-experience improvement
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters:
  - The repository strongly prefers PowerShell wrappers on Windows, but the
    cleanup-hotspot audit helper exists only as a Bash script while the docs
    present it as a general workflow tool. That creates friction for the exact
    audit/planning workflow this repo asks Windows agents to perform.
- Evidence:
  - `scripts/audit_cleanup_hotspots.sh` exists.
  - No matching `scripts/audit_cleanup_hotspots.ps1` exists.
  - `docs/INDEX.md` documents only the Bash invocation.
  - `AGENTS.md` explicitly says Windows sessions should use PowerShell wrappers
    rather than Bash workflow scripts.
- Recommended change:
  - Prefer adding a PowerShell wrapper with equivalent output semantics.
  - If parity is intentionally out of scope, document the Bash-only limitation
    explicitly in `docs/INDEX.md` and related workflow docs.
- Expected impact:
  - Lower audit friction for Windows maintainers and agents.
  - Fewer workflow contradictions in repo guidance.
- Risks / tradeoffs:
  - Low technical risk.
  - If parity is implemented, the wrapper must stay behaviorally aligned with
    the Bash version.
- Dependencies:
  - None.
- Suggested validation:
  - Compare generated markdown snapshot output between wrapper paths.
  - `devcheck.ps1` and `ci_quick.ps1`.
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What is the intended long-term boundary between in-process GUI scenarios and desktop AIV manifests?

- Evidence:
  - `src/gui_test/scenario.rs` defines in-process `GuiScenarioStep` and
    `GuiAssertion`.
  - `src/gui_test/aiv/mod.rs` defines a second semantic assertion/step contract
    for desktop AIV.
  - Both systems use similar semantic node/action concepts.
- Why this matters:
  - Future test additions may duplicate behavior or drift in supported assertion
    vocabulary if the ownership boundary is not explicit.
- Affected files/modules:
  - `src/gui_test/scenario.rs`
  - `src/gui_test/aiv/mod.rs`
  - `src/gui_test/aiv/packs/**`
- Risk if guessed incorrectly:
  - Duplicate maintenance and diverging test semantics.
- Most conservative provisional assumption:
  - Keep both systems, but avoid unifying them until the intended ownership
    boundary is documented.

### [!] What is the expected module boundary target for the native runtime after the recent performance work?

- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello.rs` already delegates to many
    submodules, but still retains a very large top-level runner and state set.
  - `docs/plans/active/runtime_performance_exec_plan.md` focuses on performance
    milestones, not final module ownership targets.
- Why this matters:
  - Some decompositions are clearly helpful, but a larger refactor could drift
    into architecture-by-preference without a documented end state.
- Affected files/modules:
  - `vendor/radiant/src/gui_runtime/native_vello.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/**`
- Risk if guessed incorrectly:
  - Expensive churn without improving the maintenance pain that actually
    matters.
- Most conservative provisional assumption:
  - Prefer small extractions around state ownership and lifecycle seams instead
    of a top-down runtime redesign.

### [!] Is the cleanup-hotspot audit helper meant to be a general workflow tool or an optional maintainer-side Bash utility?

- Evidence:
  - `docs/INDEX.md` documents the helper as part of the cleanup planning flow.
  - `AGENTS.md` tells Windows sessions not to use Bash workflow scripts.
  - Only `scripts/audit_cleanup_hotspots.sh` exists.
- Why this matters:
  - The safe recommendation differs depending on whether Windows parity is
    expected or whether documentation should explicitly mark the tool as
    Bash-only.
- Affected files/modules:
  - `scripts/audit_cleanup_hotspots.sh`
  - `docs/INDEX.md`
  - `AGENTS.md`
- Risk if guessed incorrectly:
  - Either unnecessary wrapper maintenance or continued workflow confusion.
- Most conservative provisional assumption:
  - Treat the current state as a documentation mismatch that should be resolved
    one way or the other.

## Rejected Ideas

### [-] Replace semantic GUI automation with screenshot-first desktop testing

- Why it was considered:
  - The repo has AIV support and screenshot artifacts.
- Why it was rejected:
  - The repository explicitly documents semantic-first GUI automation, and the
    current test platform is built around stable node ids and action metadata.
- Missing evidence:
  - No repo evidence that a screenshot-first oracle is desired.

### [-] Rewrite the native runtime around a new framework or async architecture

- Why it was considered:
  - The runtime is large and performance-sensitive.
- Why it was rejected:
  - The repo already has an active incremental/runtime plan and recent work in
    the current architecture. A framework rewrite would be speculative.
- Missing evidence:
  - No documented migration plan or dissatisfaction with the existing stack as a
    whole.

### [-] Promote the cleanup or perf backlogs into active implementation automatically

- Why it was considered:
  - Both backlogs remain on disk and contain actionable work.
- Why it was rejected:
  - The current user request is to refresh the improvement audit only, and the
    active-lane docs explicitly say those lanes remain parked until reopened.
- Missing evidence:
  - No user authorization to resume those lanes.
