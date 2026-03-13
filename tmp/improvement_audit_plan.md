# Evidence-Driven Improvement Audit Plan

Generated: 2026-03-13
Repository: `C:\dev\sempal`
Branch: `next`

## Repository Context

- Project purpose: Rust desktop audio sample triage tool with native GUI,
  waveform playback/editing, source management, updater flows, and semantic GUI
  automation.
  - Basis: Explicitly documented in `README.md`, `docs/ARCHITECTURE.md`,
    `docs/TEST.md`.
- Maturity level: actively iterated application with strong CI/test guardrails,
  but still carrying several oversized native GUI, runtime, and helper modules.
  - Basis: Strongly implied by `.github/workflows/ci.yml`,
    `docs/QUALITY_SCORE.md`, the active performance plan, and the current
    file-size debt allowlist.
- Primary languages/tooling: Rust 2024 workspace with Cargo, PowerShell-first
  Windows wrappers, native GUI/runtime work in `vendor/radiant`, and semantic
  GUI automation plus AIV desktop runs.
  - Basis: Explicitly documented in `AGENTS.md`, `Cargo.toml`,
    `docs/TEST.md`, `docs/gui_test_platform.md`.
- Repository shape: root app crate plus companion apps in `apps/`, support
  tools in `tools/`, and the `vendor/radiant` GUI framework submodule.
  - Basis: Explicitly documented in `Cargo.toml`, `docs/ARCHITECTURE.md`.
- Architectural boundaries: domain/controller logic lives in `src/**`,
  backend-neutral projection/action contracts in `src/app_core/**`, and native
  GUI behavior plus runtime ownership in `vendor/radiant/**`.
  - Basis: Explicitly documented in `README.md`, `docs/ARCHITECTURE.md`.
- Test strategy: deterministic Rust unit/integration coverage first, then GUI
  contract tests and AIV desktop coverage for native flows.
  - Basis: Explicitly documented in `docs/TEST.md`,
    `docs/gui_test_platform.md`.
- Canonical Windows validation commands:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Basis: Explicitly documented in `AGENTS.md`, `README.md`, `docs/TEST.md`.
- Documented priorities: correctness, maintainability, deterministic tests,
  semantic-first GUI automation, PowerShell-first Windows workflows, and
  explicit documentation on public-facing surfaces.
  - Basis: Explicitly documented in `AGENTS.md`, `docs/TEST.md`,
    `docs/gui_test_platform.md`.
- Explicit non-goals supported by repo evidence:
  - No evidence for large product-direction changes or workflow redesigns.
  - No evidence for replacing semantic GUI testing with screenshot-first
    assertions.
  - No evidence for framework swaps away from the current Rust + `radiant`
    native stack.

## ROI-Ranked Backlog

### [x] 1. Split native-shell automation snapshot building by surface and shared helper layer

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The automation snapshot is the semantic contract for GUI tooling, CLI
    runners, and AIV. Splitting it reduced review risk in the highest-value GUI
    contract surface.
- Evidence:
  - The old `vendor/radiant/src/gui/native_shell/state/automation.rs` mixed all
    panel families and exceeded the file-size budget.
  - `docs/gui_test_platform.md` and `src/gui_test/aiv/**` rely on stable node
    ids and action metadata from this path.
- Recommended change:
  - Completed: split by panel family and helper layer while preserving stable
    automation ids.
- Expected impact:
  - Lower GUI automation regression risk and better locality for future surface
    changes.
- Risks / tradeoffs:
  - Completed with moderate refactor churn in a high-touch file.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo test -p radiant automation::tests -- --nocapture`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `ee5b868e`
  - Validation: targeted `radiant` automation tests plus Windows `devcheck` and
    `ci_quick`

### [x] 2. Split browser-row layout/windowing helpers out of the current multi-purpose helper file

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Browser autoscroll/focus work had concentrated in one mixed-responsibility
    helper file; the split reduced churn coupling around row-window math.
- Evidence:
  - The old `vendor/radiant/src/gui/native_shell/state/browser_rows.rs` mixed
    viewport math, truncation, toolbar/scrollbar geometry, and row styling.
  - Browser interaction regressions repeatedly landed in this area.
- Recommended change:
  - Completed: split viewport/windowing math, truncation, geometry, and visual
    helpers into focused modules.
- Expected impact:
  - Lower chance of reintroducing browser scroll/focus regressions.
- Risks / tradeoffs:
  - Completed with moderate refactor churn in native GUI layout helpers.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo test -p radiant browser_rows -- --nocapture`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `943f908b`
  - Validation: targeted `radiant` browser-row tests plus Windows `devcheck`
    and `ci_quick`

### [x] 3. Prune stale file-size-budget allowlist entries and refresh debt-tracking docs

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The repo uses the file-size allowlist and scorecard as planning sources of
    truth. Stale entries overstate current debt and can misdirect future audit
    work.
- Evidence:
  - `docs/file_size_budget_allowlist.txt` still references paths that no longer
    exist:
    - `src/app/controller/ui/clipboard_paste/source_job.rs`
    - `src/app_core/native_bridge/projection_cache/projection_key.rs`
    - `vendor/radiant/src/gui/native_shell/state/hit_testing.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/runtime_render.rs`
  - The allowlist also still lists already-split historical files such as the
    old automation/browser-row hotspots that no longer exist in their former
    paths.
- Recommended change:
  - Remove nonexistent entries from `docs/file_size_budget_allowlist.txt`.
  - Refresh related debt-tracking references in `docs/QUALITY_SCORE.md` and any
    audit docs that still describe the older hotspot set.
- Expected impact:
  - More trustworthy cleanup planning and less audit drift.
- Risks / tradeoffs:
  - Low technical risk; only small documentation/guardrail configuration churn.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `f15ba421` (`docs(quality): prune stale file size debt entries`)
  - Assumption used: removing nonexistent or now-under-budget allowlist entries does not change guardrail intent; it only realigns the debt ledger with the live tree.
  - Validation: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 4. Remove `missing_docs` suppressions from the public GUI contract surfaces

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - These enums and assertion types are public contract surfaces for host tests,
    GUI tooling, and automation. Undocumented variants slow safe extension and
    directly contradict the repo’s documentation rules.
- Evidence:
  - `src/app_core/actions/catalog.rs` still has `#[allow(missing_docs)]` on
    `GuiActionKind`, `GuiSurface`, `GuiEffectClass`, and `GuiCoverageLayer`.
  - `src/gui_test/scenario.rs` still has `#[allow(missing_docs)]` on
    `GuiScenarioStep` and `GuiAssertion`.
  - `Cargo.toml` enables `missing_docs = "deny"` at the workspace lint level,
    and `AGENTS.md` explicitly requires clear docs for public-facing objects.
- Recommended change:
  - Replace the suppressions with concise docs for each enum and non-obvious
    variant, including constraints where behavior is subtle.
- Expected impact:
  - Better discoverability for GUI action/test contracts.
  - Less ambiguity when extending automation or AIV packs.
- Risks / tradeoffs:
  - Low technical risk; may expose minor naming issues while documenting.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo doc -p sempal --no-deps`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `8aa2a333` (`docs(gui): document public test contracts`)
  - Assumption used: concise one-line variant docs are sufficient here because the surrounding module docs and catalog metadata already carry the deeper semantics for each action family.
  - Validation: `cargo doc -p sempal --no-deps`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 5. Split native-shell frame-build browser, chrome, and overlay builders by responsibility

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Recent browser/focus/render fixes have concentrated in the retained
    frame-build layer. The three biggest builder files still mix layout
    geometry, style decisions, borders, hover/focus overlays, and per-surface
    rendering details in ways that make visual regressions hard to isolate.
- Evidence:
  - `vendor/radiant/src/gui/native_shell/state/frame_build/browser.rs` is 683
    lines and mixes row drawing, rating indicators, inline tag chips,
    scrollbar rendering, and browser frame composition.
  - `vendor/radiant/src/gui/native_shell/state/frame_build/chrome.rs` is 608
    lines and mixes shell surfaces, borders, top bar controls, sidebar
    rendering, and chrome text/layout work.
  - `vendor/radiant/src/gui/native_shell/state/frame_build/overlay.rs` is 673
    lines and mixes search editor overlays, row hover fills, focus overlays,
    prompt/options/progress overlays, and drag overlay rendering.
- Recommended change:
  - Extract focused modules for browser rows/indicators/scrollbars, shell
    surfaces/borders/top-bar/sidebar chrome, and overlay families
    (hover/focus/dialogs).
  - Keep each file aligned to one visual responsibility.
- Expected impact:
  - Lower risk when fixing browser and overlay rendering issues.
  - Easier targeted testing and review of native-shell visual changes.
- Risks / tradeoffs:
  - Moderate refactor risk in a sensitive render path.
- Dependencies:
  - None.
- Suggested validation:
  - Existing `radiant` native-shell tests
  - GUI contract/AIV runs for browser and waveform flows
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `8b94b016` (`refactor(gui): split frame build surfaces`)
  - Assumption used: a mechanical module split is the safe first step here; preserving existing function names and call sites reduces render-path regression risk while still shrinking the responsibility surface.
  - Validation: `cargo test -p radiant frame_build -- --nocapture`, `cargo test -p radiant overlay -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 6. Add direct helper coverage and smaller internal seams for waveform line rasterization

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - The waveform line renderer is dense, math-heavy, and performance-sensitive.
    Subtle interpolation or blending regressions are expensive to diagnose when
    the logic lives in one large raster file.
- Evidence:
  - `src/waveform/render/paint/lines.rs` is 503 lines.
  - It currently owns sample interpolation, supersampling, Catmull-Rom
    interpolation, anti-aliased line stepping, pixel blending, and final image
    styling in one unit.
- Recommended change:
  - Split interpolation, supersampling, line stepping, and blend helpers into
    smaller internal modules or helper functions with direct deterministic
    tests.
  - Add boundary tests for zero-sized rasters, end-of-buffer interpolation,
    vertical/steep lines, and alpha/coverage edge cases.
- Expected impact:
  - Better confidence in a hot render path.
  - Faster diagnosis when waveform visuals regress.
- Risks / tradeoffs:
  - Moderate refactor effort in performance-sensitive code.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted waveform render tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `a4c7f571` (`test(waveform): split line raster helpers`)
  - Assumption used: splitting interpolation and raster helpers behind the existing `WaveformRenderer` entrypoints is the safest seam because it improves direct coverage without changing the external render API.
  - Validation: `cargo test waveform::render::paint::lines -- --nocapture`, `cargo test waveform::render:: -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 7. Split waveform pointer routing into hit-resolution, deselection, and drag-mode helpers

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: M
- Why it matters:
  - Waveform pointer behavior is a repeated correctness hotspot. The current
    router is an ordered chain of gesture precedence rules, deselection logic,
    drag-mode mapping, and immediate/deferred emission policy in one file.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing.rs` is
    544 lines.
  - `waveform_action_from_pointer` delegates through a long ordered sequence of
    early-return helpers for primary actions, new-selection actions, and clear
    actions before defaulting to cursor/seek/selection behavior.
  - The same file also owns resize-handle hover checks, selection/edit drag
    routing, drag-mode mapping, click-slop behavior, and press-emission policy.
- Recommended change:
  - Separate gesture hit-resolution from action emission and drag-mode policy.
  - Keep deselection/new-selection behavior in focused helpers with
    table-driven tests for precedence and overlap cases.
- Expected impact:
  - Lower regression risk in zoomed waveform interactions and mixed mark flows.
  - Clearer reasoning about gesture precedence.
- Risks / tradeoffs:
  - Moderate refactor risk because gesture ordering must remain stable.
- Dependencies:
  - None.
- Suggested validation:
  - Existing `radiant` waveform pointer/drag tests
  - AIV waveform interaction pack
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `1fbf6361` (`refactor(waveform): split pointer routing helpers`)
  - Assumption used: keeping the public routing surface in `input.rs` unchanged while splitting only the internal helper families is the lowest-risk way to improve readability in this precedence-sensitive path.
  - Validation: `cargo test -p radiant waveform_pointer -- --nocapture`, `cargo test -p radiant waveform_drag -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 8. Continue decomposing the native Vello runtime around lifecycle, input, and render ownership

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: L
- Why it matters:
  - The root runtime still holds a very large runner struct with startup,
    pointer/input state, retained scenes, text input, clipboard, and render
    ownership in one orchestration file. That keeps lifecycle and input/render
    interactions expensive to audit.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello.rs` is 756 lines.
  - `NativeVelloRunner` still stores window/surface/render objects, retained
    segment scene caches, motion fingerprints, layout/shell state, pointer drag
    state, text input/editor state, clipboard state, and repaint flags together
    in one top-level struct.
  - The active performance plan in
    `docs/plans/active/runtime_performance_exec_plan.md` identifies this runtime
    path as a long-running responsiveness hotspot.
- Recommended change:
  - Extract cohesive ownership modules for startup/window reveal sequencing,
    transient input/drag state, retained render state, and text/clipboard state.
  - Keep the root runner focused on orchestration.
- Expected impact:
  - Better auditability for lifecycle bugs and input/render races.
  - Lower maintenance cost for future runtime changes.
- Risks / tradeoffs:
  - Highest refactor risk in this backlog.
  - Must preserve current performance and event-ordering wins.
- Dependencies:
  - None, but should follow smaller high-confidence items first.
- Suggested validation:
  - Existing `radiant` runtime tests
  - GUI contract/AIV suites
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `vendor/radiant` `f09f20d1` (`refactor(runtime): split native vello action and text helpers`)
  - Assumption used: extracting action-classification and text/clipboard runtime helpers first is the safest runtime split because these seams already behave like shared service surfaces across the event, input, and text modules.
  - Validation: `cargo test -p radiant runtime_core -- --nocapture`, `cargo test -p radiant key_bindings -- --nocapture --test-threads=1`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - Validation note: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` reached a pre-existing guardrail failure in `src/gui_test/aiv/packs/cases.rs` exceeding the file-size budget; item 8 itself compiled and passed the runtime-focused checks.

### [x] 9. Split updater-helper UI orchestration into background tasks, state transitions, and view projection

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The updater companion UI mixes release loading, update execution, log
    buffering, background receivers, selection movement, and panel-model
    projection in one file. That is a user-facing state machine with more
    mixed responsibilities than the repo’s structure guidelines allow.
- Evidence:
  - `apps/updater-helper/src/ui.rs` is 517 lines.
  - `UpdateNativeBridge` owns release polling, release selection, update start,
    log buffering, background progress/result consumption, and app-model
    projection helpers in one unit.
- Recommended change:
  - Split background task management, reducer-like UI state transitions, and
    view-model projection into focused modules.
  - Add targeted tests for state transitions where practical.
- Expected impact:
  - Safer maintenance of the update/install companion flow.
  - Better readability around background state changes.
- Risks / tradeoffs:
  - Moderate refactor effort in a smaller but user-visible app path.
- Dependencies:
  - None.
- Suggested validation:
  - Updater-helper targeted tests if present or added
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: `420eef61` (`refactor(updater): split updater helper ui flow`)
  - Assumption used: keeping the `UpdateNativeBridge` data shape stable while moving only task, state, and projection methods into submodules is the lowest-risk way to split this user-facing state machine.
  - Validation: `cargo test -p sempal-updater-helper -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [x] 10. Split compute-worker batch execution from queue loop and deferred-finalization policy

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The compute worker manages shutdown/cancel polling, batch pop timing,
    settings capture, connection maps, panic recovery, decoded-batch grouping,
    immediate finalization, and deferred update flushing in one threaded path.
- Evidence:
  - `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs`
    is 453 lines.
  - `run_compute_worker`, `process_batch`, `process_batch_work`, and
    `run_work_item` share queue orchestration, DB connection management, and
    finalization concerns in one file.
- Recommended change:
  - Separate worker-loop orchestration, work-item execution, and deferred update
    flushing/finalization helpers.
  - Add focused tests for panic recovery and deferred flush behavior if missing.
- Expected impact:
  - Easier reasoning about threaded job execution and cancellation behavior.
  - Lower regression risk when changing analysis job execution.
- Risks / tradeoffs:
  - Moderate refactor risk in threaded code.
- Dependencies:
  - None.
- Suggested validation:
  - Existing analysis-job tests
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No
- Completion:
  - Date: 2026-03-13
  - Commit: pending current item commit
  - Assumption used: preserving the top-level worker loop while only extracting execution and deferred-finalization helpers is enough to reduce ownership mixing without changing queue semantics.
  - Validation: `cargo test job_claim -- --nocapture`, `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`, `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### [ ] 11. Separate waveform zoom-cache core behavior from telemetry bookkeeping

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - Core cache behavior is currently interleaved with global telemetry counters
    and emission helpers, which makes eviction/locking behavior harder to read
    and change safely.
- Evidence:
  - `src/waveform/zoom_cache.rs` is 489 lines.
  - The file mixes `WaveformZoomCache` and `CacheInner` behavior with many
    global telemetry `AtomicU64`s, `OnceLock` state, and emission helpers.
- Recommended change:
  - Move telemetry counters/emission into a dedicated helper module or type.
  - Keep the cache file focused on keying, lookup, insertion, eviction, and
    poison recovery.
- Expected impact:
  - Clearer ownership between cache semantics and observability.
  - Easier future cache changes.
- Risks / tradeoffs:
  - Moderate churn in a performance-sensitive path.
- Dependencies:
  - None.
- Suggested validation:
  - Existing waveform cache tests and perf guardrails
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 12. Separate pure audio-option normalization from controller-side probing and mutation

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The file already has a clean pure normalization seam, but it still mixes
    probe policy, warning construction, settings mutation, UI projection,
    persistence-related state updates, and player-facing refresh behavior in one
    controller unit.
- Evidence:
  - `src/app/controller/playback/audio_options.rs` is 445 lines.
  - It combines `normalize_audio_options`, output/input refresh paths, channel
    normalization, warning construction, and selection/apply helpers in one
    file.
- Recommended change:
  - Keep pure normalization/probe helpers separate from controller mutation and
    UI projection helpers.
  - Introduce small shared helpers for output/input view projection and warning
    handling.
- Expected impact:
  - Easier targeted testing of audio settings behavior.
  - Lower coupling between probing policy and controller mutation.
- Risks / tradeoffs:
  - Moderate refactor touching settings and player rebuild flows.
- Dependencies:
  - None.
- Suggested validation:
  - Existing audio-option tests
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 13. Split oversized test and fixture catalogs into domain-focused modules

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - Large test hubs and case catalogs reduce discoverability and make it harder
    to add or review regressions without touching unrelated behaviors.
- Evidence:
  - `src/gui_test/aiv/packs/cases.rs` is 555 lines.
  - `src/app_core/controller/tests.rs` is 562 lines.
  - `src/app_core/native_bridge/tests/projection_cache.rs` is 609 lines.
  - `vendor/radiant/src/gui/native_shell/state/tests/browser_rows.rs` is 860
    lines.
- Recommended change:
  - Split these files by domain or behavior family instead of continuing to grow
    them as omnibus catalogs.
- Expected impact:
  - Easier navigation and lower review friction for regression additions.
- Risks / tradeoffs:
  - Medium confidence because this is maintainability debt, not an immediate
    correctness bug.
- Dependencies:
  - None.
- Suggested validation:
  - Existing targeted test suites plus `ci_quick.ps1`
- Product clarification required: No

### [ ] 14. Add Windows parity for the cleanup-hotspot audit helper or document its Bash-only status precisely

- Classification: Developer-experience improvement
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters:
  - The repo tells Windows agents to use PowerShell wrappers, but the cleanup
    hotspot helper used for ROI planning still exists only as Bash and is
    documented as a generic workflow tool.
- Evidence:
  - `scripts/audit_cleanup_hotspots.sh` exists.
  - No matching `scripts/audit_cleanup_hotspots.ps1` exists.
  - `docs/INDEX.md` documents only the Bash invocation.
  - `AGENTS.md` explicitly says Windows sessions should use PowerShell wrappers
    rather than Bash workflow scripts.
- Recommended change:
  - Prefer adding a PowerShell wrapper with matching output semantics.
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
  - Compare generated hotspot snapshots across wrapper paths
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What is the intended long-term boundary between in-process GUI scenarios and desktop AIV manifests?

- Evidence:
  - `src/gui_test/scenario.rs` defines in-process `GuiScenarioStep` and
    `GuiAssertion`.
  - `src/gui_test/aiv/mod.rs` defines a second semantic step/assertion contract
    for desktop AIV.
  - Both systems operate on similar semantic node/action concepts.
- Why this matters:
  - Without a documented boundary, future test additions may duplicate behavior
    or drift in supported assertion vocabulary.
- Affected files/modules:
  - `src/gui_test/scenario.rs`
  - `src/gui_test/aiv/mod.rs`
  - `src/gui_test/aiv/packs/**`
- Risk if guessed incorrectly:
  - Duplicate maintenance and diverging semantic test contracts.
- Most conservative provisional assumption:
  - Keep both systems for now and avoid unifying them until the intended
    ownership boundary is documented.

### [!] What is the intended end-state boundary for the native runtime after the performance redesign?

- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello.rs` already delegates to many
    submodules but still retains a very large top-level runner and state set.
  - `docs/plans/active/runtime_performance_exec_plan.md` focuses on performance
    milestones, not the desired final module ownership shape.
- Why this matters:
  - Some decompositions are clearly useful, but a larger runtime refactor could
    drift into architecture-by-preference without a documented target.
- Affected files/modules:
  - `vendor/radiant/src/gui_runtime/native_vello.rs`
  - `vendor/radiant/src/gui_runtime/native_vello/**`
- Risk if guessed incorrectly:
  - High-cost churn without improving the real maintenance pain.
- Most conservative provisional assumption:
  - Prefer incremental extractions around state ownership and lifecycle seams
    instead of a top-down runtime redesign.

### [!] Is the cleanup-hotspot audit helper meant to be a general workflow tool or an optional Bash-only maintainer utility?

- Evidence:
  - `docs/INDEX.md` presents the helper as part of the cleanup planning flow.
  - `AGENTS.md` tells Windows sessions not to use Bash workflow scripts.
  - Only `scripts/audit_cleanup_hotspots.sh` exists.
- Why this matters:
  - The safe recommendation differs depending on whether Windows parity is
    expected or whether the docs should explicitly mark the helper as Bash-only.
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
  - The repo already has an active incremental performance plan and recent work
    in the current architecture. A framework rewrite would be speculative.
- Missing evidence:
  - No documented migration plan or dissatisfaction with the existing stack as a
    whole.

### [-] Treat every oversized test file as an immediate correctness problem

- Why it was considered:
  - Several test hubs exceed the file-size budget.
- Why it was rejected:
  - The repo evidence supports splitting them for maintainability, but not every
    large test file clearly blocks current correctness work.
- Missing evidence:
  - No evidence that a broad test-only churn pass is currently more valuable
    than the targeted runtime/native-shell items above.
