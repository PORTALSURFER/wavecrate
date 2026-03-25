# Improvement Audit Plan

Generated: 2026-03-25
Observed commit: `8056af85`
Status: Phase 2 execution approved on 2026-03-25; implement items sequentially in ranked order unless blocked.

## Scope

- This document supersedes the previous execution record that lived at this path.
- Findings are ranked in strict execution order by expected ROI for the live tree observed on 2026-03-25.
- Recommendations stay inside repository-supported direction. Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Decision Log

- 2026-03-25: The user approved Phase 2 sequential implementation of the ranked backlog.
- 2026-03-25: Direct tagging to `KEEP_3` was clarified to represent the fourth keep state and must also promote the sample into the locked state.
- 2026-03-25: The stateless handoff authority follows `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md`, with `docs/plans/index.md` serving as the historical index.
- 2026-03-25: Public release docs should use conservative wording: Windows is the primary supported platform, while Linux and macOS assets are also published.
- 2026-03-25: Desktop AIV should remain local-only for now, with any future CI promotion starting from a tiny non-blocking smoke subset.

## Repository Context

- Project purpose: Explicitly documented. [README.md](/C:/dev/sempal/README.md) and [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) describe Sempal as a realtime-oriented Rust desktop tool for triaging, auditioning, editing, and curating local audio samples.
- Maturity level: Explicitly documented. [README.md](/C:/dev/sempal/README.md) warns that the app is early alpha and can modify or delete sample-library files.
- Primary languages / frameworks / tooling: Explicitly documented. [Cargo.toml](/C:/dev/sempal/Cargo.toml) defines a Rust 2024 workspace with the vendored `radiant` GUI/runtime layer plus helper apps and support tools.
- Repository shape: Explicitly documented. [docs/ARCHITECTURE.md](/C:/dev/sempal/docs/ARCHITECTURE.md) splits domain/controller logic under `src/`, GUI behavior under `vendor/radiant/`, and support apps/tools under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. [README.md](/C:/dev/sempal/README.md) and [docs/ARCHITECTURE.md](/C:/dev/sempal/docs/ARCHITECTURE.md) keep domain state and UI intent in `src`, while `vendor/radiant` owns widget behavior, layout, input routing, and render coordination.
- Test strategy: Strongly implied by code/docs. [docs/TEST.md](/C:/dev/sempal/docs/TEST.md) and the source tree emphasize deterministic Rust unit/module tests, targeted GUI contract tests, and optional desktop-AIV loops.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Documented priorities: Explicitly documented. [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) prioritizes responsiveness, non-blocking execution, predictability, reversibility, and data integrity.
- Explicit non-goals: Explicitly documented. [docs/design_principles.md](/C:/dev/sempal/docs/design_principles.md) says Sempal is not a DAW replacement, cloud platform, social network, or attention-retention product.

## Audit Notes

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` passed on the observed tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` passed, and `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all` reported `OK (654 files checked)`.
- The broader hotspot snapshot still shows live debt outside the currently enforced allowlist: [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) reports 11 over-budget Rust files, 85 heuristic test-gap hotspots, and one remaining `#[allow(dead_code)]` suppression.
- The file-size guardrail appears to have a live blind spot: [scripts/check_file_size_budget.ps1](/C:/dev/sempal/scripts/check_file_size_budget.ps1) claims to scan `vendor/radiant/src`, but `git ls-files -- vendor/radiant/src` returns `0` from the superproject while `vendor/radiant` is an active submodule containing 295 Rust source files.

## Intent Boundaries

- What the repo clearly is: a Rust desktop application for listening to, navigating, editing, and curating local sample libraries with strong emphasis on responsiveness and reversible workflows.
- What the repo appears to be moving toward: Strongly implied by code/docs. Tighter guardrails around stateless-agent handoff, stronger GUI contract coverage, continued migration to `app_core` seams, and broader native-runtime validation.
- What is merely possible but unsupported: shipping desktop AIV in default CI today, broad `app_core` action-model redesigns, or major GUI/runtime ownership changes that contradict the thin-adapter boundary.

## Ordered Backlog

### 1. [x] Fix the file-size guardrail blind spot for `vendor/radiant` submodule files

- Classification: Bug fix
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the repository treats file-size discipline as an enforced quality guardrail, but the current script silently misses the vendored GUI/runtime code it claims to cover.
- Evidence:
  - [scripts/check_file_size_budget.ps1](/C:/dev/sempal/scripts/check_file_size_budget.ps1#L7) says it checks `vendor/radiant/src`.
  - [scripts/check_file_size_budget.ps1](/C:/dev/sempal/scripts/check_file_size_budget.ps1#L39) scopes to `vendor/radiant/src`.
  - [scripts/check_file_size_budget.ps1](/C:/dev/sempal/scripts/check_file_size_budget.ps1#L55) discovers files via `git ls-files -- $scopePaths`.
  - Running `git ls-files -- vendor/radiant/src` from the superproject returned `0`, while `vendor/radiant` is an initialized submodule and `Get-ChildItem vendor/radiant/src -Recurse -Filter *.rs` found `295` Rust files.
  - Large live files exist under the skipped tree, including `vendor/radiant/src/app/hotkeys.rs` (784 lines) and `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` (1055 lines).
- Recommended change: make the file-budget scripts enumerate `vendor/radiant` files from the submodule checkout instead of relying on the superproject index, and add regression fixtures proving that oversized `vendor/radiant/src` files fail the check.
- Expected impact: restores trust in a repo-advertised guardrail and makes `docs/QUALITY_SCORE.md` more honest.
- Risks / tradeoffs: low; the main risk is double-scanning or incorrectly handling uninitialized submodules.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `bash scripts/check_file_size_budget.sh --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `b2ee3235`
- Assumptions: Keep the full guardrail green by allowlisting the current oversized vendored legacy files explicitly instead of silently excluding the entire nested repo.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_script_guardrails.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 2. [x] Resynchronize the stateless handoff docs and active plan status markers

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: this repository explicitly depends on `AGENTS.md`, `MEMORY.md`, `docs/README.md`, and plan files to reorient stateless agents, but those entry points currently disagree about what has shipped and which plan is live.
- Evidence:
  - [docs/README.md](/C:/dev/sempal/docs/README.md#L27) still says [tmp/improvement_audit_plan.md](/C:/dev/sempal/tmp/improvement_audit_plan.md) is awaiting Phase 2 confirmation.
  - [docs/plans/index.md](/C:/dev/sempal/docs/plans/index.md#L6) still presents `active/runtime_performance_exec_plan.md` as the current source of truth, while [docs/plans/index.md](/C:/dev/sempal/docs/plans/index.md#L15) also says the improvement audit Phase 2 completed on 2026-03-25.
  - [docs/plans/active/gui_test_platform_exec_plan.md](/C:/dev/sempal/docs/plans/active/gui_test_platform_exec_plan.md#L43) still lists prompt/waveform AIV coverage and `ci_quick` promotion as pending.
  - [docs/gui_test_platform.md](/C:/dev/sempal/docs/gui_test_platform.md#L149) says `ci_quick` already includes the GUI contract lane, and [docs/gui_test_platform.md](/C:/dev/sempal/docs/gui_test_platform.md#L169) says prompt/waveform/update flows already run through desktop AIV.
- Recommended change: resync the repo-entry docs and plan index around the current lane, and explicitly distinguish active source-of-truth documents from retained historical plans.
- Expected impact: reduces agent/operator confusion and makes future audit handoff cheaper.
- Risks / tradeoffs: low; the main risk is overwriting intentionally historical wording without preserving history elsewhere.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `19455ebb`
- Assumptions: `AGENTS.md`, `MEMORY.md`, and `docs/plans/active/todo.md` remain the wake-up authority, while `docs/plans/index.md` is the retained plan catalog.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 3. [x] Correct the public release-platform docs to match the live release workflow

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the top-level README is user-facing release guidance, and it currently understates what the repository actually publishes.
- Evidence:
  - [README.md](/C:/dev/sempal/README.md#L18) says GitHub Releases publish Windows binaries only.
  - [release-build.yml](/C:/dev/sempal/.github/workflows/release-build.yml#L19) builds Windows, Linux x86_64, Linux aarch64, macOS x86_64, and macOS aarch64 assets.
- Recommended change: update the Downloads section to describe the actual release matrix, using conservative wording if non-Windows assets are published but not yet considered equally supported.
- Expected impact: improves user trust and reduces avoidable installation confusion.
- Risks / tradeoffs: low; wording should avoid promising support guarantees the maintainers do not want to make.
- Dependencies: none
- Suggested validation:
  - review [README.md](/C:/dev/sempal/README.md) against [release-build.yml](/C:/dev/sempal/.github/workflows/release-build.yml)
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
- Product clarification required: No
- Completed: 2026-03-25
- Commit: `a1c8b659`
- Assumptions: Release assets for Linux and macOS are published artifacts, but Windows remains the primary explicitly supported release platform until docs say otherwise.
- Validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_markdown_links.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Plan order deviation: none

### 4. [ ] Add crash-recovery regression coverage for drop-target copy/move failures after durable journal stages

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: drop-target transfers mutate both filesystem state and multiple databases, and the repo’s recovery contract explicitly depends on staged journal semantics that are only partially exercised today.
- Evidence:
  - [docs/file_ops_journal_recovery.md](/C:/dev/sempal/docs/file_ops_journal_recovery.md) defines the `Intent -> Staged -> TargetDb -> SourceDb` recovery contract.
  - [worker.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/worker.rs#L268) commits target DB state before finalizing staged copies/moves.
  - [worker.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/worker.rs#L307) and [worker.rs](/C:/dev/sempal/src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/worker.rs#L375) return errors after durable stage advancement, relying on later recovery rather than immediate rollback.
  - Existing coverage under [transfer/workflow.rs](/C:/dev/sempal/src/app/controller/tests/drag_drop_drop_targets/transfer/workflow.rs) covers happy paths, target-DB lock failures, and source-cleanup failure, but not post-`TargetDb` copy-finalize failure or post-`SourceDb` move-finalize failure invariants.
- Recommended change: add focused tests that simulate failures after `TargetDb` and `SourceDb` stage transitions and assert the expected staged-file, journal-row, and DB-state outcomes.
- Expected impact: hardens one of the repo’s most data-integrity-sensitive workflows without changing design direction.
- Risks / tradeoffs: medium; fixtures will need careful control over staged files and DB state to keep the tests deterministic.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test drag_drop_drop_targets --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 5. [ ] Deepen GUI contract harness tests around scenario assertions and automation target resolution

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the GUI test runner and automation helpers are the contract surface for semantic GUI assertions, the CLI, and desktop AIV targeting, but their error-handling coverage is much thinner than their branch surface.
- Evidence:
  - [runner.rs](/C:/dev/sempal/src/gui_test/runner.rs#L129) implements nine assertion branches.
  - [runner.rs](/C:/dev/sempal/src/gui_test/runner.rs#L231) only has two local tests, covering root capture and one `NodePresent` path.
  - [automation.rs](/C:/dev/sempal/src/gui_test/automation.rs#L42) resolves semantic targets for both the CLI and AIV wrappers, and [automation.rs](/C:/dev/sempal/src/gui_test/automation.rs#L68) parses snapshots from artifact bundles.
  - [automation.rs](/C:/dev/sempal/src/gui_test/automation.rs#L102) only tests one happy-path root target.
- Recommended change: add table-driven runner tests for each assertion type plus missing-node failure cases, and add automation-helper tests for missing `automation_snapshot` payloads, malformed artifacts, and absent node ids.
- Expected impact: faster localization for GUI contract regressions without needing desktop automation to fail first.
- Risks / tradeoffs: low; these tests should stay focused on the existing semantic API rather than growing new fixture infrastructure.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test gui_test:: -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Add explicit Windows/root-path sanitization regression tests across the audio loader and source DB

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: path sanitization is an explicit safety boundary for a Windows-first desktop app, but current tests only pin a subset of the rejected path forms.
- Evidence:
  - [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L414) rejects `ParentDir`, `RootDir`, and `Prefix(_)`.
  - [audio_loader/tests.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/tests.rs#L82) only tests parent-dir rejection and normal relative-path acceptance.
  - [sample_sources/db/util.rs](/C:/dev/sempal/src/sample_sources/db/util.rs#L35) rejects absolute, rooted, and prefixed paths.
  - [sample_sources/db/util.rs](/C:/dev/sempal/src/sample_sources/db/util.rs#L72) only tests parent-dir, empty, and `.` cleanup.
- Recommended change: add targeted regression tests for rooted paths and Windows-style prefixed paths, keeping platform-gated expectations where `Component::Prefix(_)` only exists on Windows.
- Expected impact: locks down a small but important safety contract at very low cost.
- Risks / tradeoffs: low; platform-specific assertions need careful `cfg` gating to avoid brittle cross-platform failures.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test audio_loader --lib`
  - targeted `cargo test sample_sources::db::util --lib`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No

### 7. [ ] Add direct tests for the updater-helper CLI parser and headless apply path

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: S-M
- Why it matters: the updater helper is part of the shipped update flow, but its top-level parser and headless execution path are largely untested.
- Evidence:
  - [apps/updater-helper/src/main.rs](/C:/dev/sempal/apps/updater-helper/src/main.rs#L21) owns the `try_main` and `run_headless` entry flow.
  - [apps/updater-helper/src/main.rs](/C:/dev/sempal/apps/updater-helper/src/main.rs#L40) implements non-trivial CLI parsing, target/platform defaults, and help/error behavior.
  - [apps/updater-helper/src/ui/tests.rs](/C:/dev/sempal/apps/updater-helper/src/ui/tests.rs) only contains two focused UI-state tests.
  - [release-build.yml](/C:/dev/sempal/.github/workflows/release-build.yml#L121) signs and packages `sempal-updater.exe`.
- Recommended change: extract or directly test the parser/headless seams so help text, required args, platform defaults, and `--headless` execution behavior are covered without requiring the GUI layer.
- Expected impact: protects a shipped release/update path with relatively little test code.
- Risks / tradeoffs: low to medium; parser tests should avoid coupling too tightly to exact help-text formatting.
- Dependencies: none
- Suggested validation:
  - `cargo test -p updater-helper -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 8. [ ] Split `src/app/controller/playback/audio_loader/stages.rs` by its existing stage boundaries

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: the audio loader already has clear IO/decode/stretch/finalize stage seams, but one over-budget file still owns all of them plus test hooks and path sanitization.
- Evidence:
  - [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) lists `src/app/controller/playback/audio_loader/stages.rs` at 437 lines.
  - [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L45) `load_audio_inner`, [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L78) `load_io_stage`, [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L148) `decode_stage`, [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L171) `run_stretch_stage`, [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L311) `build_transient_result`, [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L333) `read_bytes_chunked_with_stale_check`, and [audio_loader/stages.rs](/C:/dev/sempal/src/app/controller/playback/audio_loader/stages.rs#L414) `ensure_safe_relative_path` already reflect separate responsibilities.
- Recommended change: keep the public loader API stable but extract focused stage helpers/modules for IO/path safety, decode/stretch, and transient/finalize behavior.
- Expected impact: restores cohesion in a production hot path without changing semantics.
- Risks / tradeoffs: medium; refactor churn around a worker path needs to preserve stale-request handling exactly.
- Dependencies: item 6 is a good safety-first precursor
- Suggested validation:
  - targeted `cargo test audio_loader --lib`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 --all`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 9. [ ] Consolidate keep-lock tagging semantics and remove the duplicated direct-tag path

- Classification: Product-definition gap
- Confidence: Medium
- ROI: Medium
- Effort: S-M
- Why it matters: lock state is part of the sample-safety model, but direct keep-tagging and incremental rating adjustment currently imply different semantics and one branch appears effectively dead.
- Evidence:
  - [tagging/mod.rs](/C:/dev/sempal/src/app/controller/playback/tagging/mod.rs#L82) implements `tag_selected(...)` directly.
  - [tagging/mod.rs](/C:/dev/sempal/src/app/controller/playback/tagging/mod.rs#L96) skips locked entries, yet [tagging/mod.rs](/C:/dev/sempal/src/app/controller/playback/tagging/mod.rs#L99) still computes `target_locked = ctx.entry.locked && target == KEEP_3`, which cannot currently become `true`.
  - [selection_ops/tags.rs](/C:/dev/sempal/src/app/controller/library/wavs/selection_ops/tags.rs#L56) already has a shared `set_sample_tag_for_source(...)` path that preserves lock state for `KEEP_3`.
  - [rating_logic.rs](/C:/dev/sempal/src/app/controller/tests/rating_logic.rs#L170) covers lock promotion through `adjust_selected_rating(1)`, but not the direct-tagging path.
- Recommended change: document the clarified rule that direct `KEEP_3` tagging must also lock, route both direct-tag and incremental-rating flows through one helper, and remove the dead branch.
- Expected impact: makes keep/lock semantics explicit and reduces drift in a safety-sensitive workflow.
- Risks / tradeoffs: medium; the change must preserve existing keep/lower-state behavior outside the clarified top-keep lock rule.
- Dependencies: none
- Suggested validation:
  - targeted rating/tagging tests
  - targeted browser action tests that exercise direct tag and incremental rating flows
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No

### 10. [ ] Align CODEOWNERS and historical-path guardrail docs with the current module tree

- Classification: Documentation gap
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters: the repo says CODEOWNERS mirrors the architecture map, but several entries still point at historical or wrong paths.
- Evidence:
  - [docs/ARCHITECTURE.md](/C:/dev/sempal/docs/ARCHITECTURE.md#L37) documents `src/external_drag/` as a directory.
  - [CODEOWNERS](/C:/dev/sempal/.github/CODEOWNERS#L22) still lists `/src/gui_app/` and [CODEOWNERS](/C:/dev/sempal/.github/CODEOWNERS#L24) `/src/legacy_runtime/`, which are not current live paths in this tree.
  - [CODEOWNERS](/C:/dev/sempal/.github/CODEOWNERS#L39) points at `/src/external_drag.rs` instead of the live `src/external_drag/` directory.
  - [docs/INDEX.md](/C:/dev/sempal/docs/INDEX.md#L103) and [docs/INDEX.md](/C:/dev/sempal/docs/INDEX.md#L112) still mention `crate::legacy_runtime::` and `crate::gui_app::` as historical tokens.
- Recommended change: update CODEOWNERS to the current module tree and clarify historical path names in guardrail docs only where they are still intentionally reserved for boundary checks.
- Expected impact: better reviewer-routing hygiene and less confusion for future ownership changes.
- Risks / tradeoffs: low; because one owner currently covers everything, the main benefit is correctness and future maintainability.
- Dependencies: none
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/check_codeowners_coverage.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_docs_index.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. What success bar should justify promoting any desktop-AIV subset into CI?

- Evidence:
  - [docs/gui_test_platform.md](/C:/dev/sempal/docs/gui_test_platform.md#L187) still documents `SetForegroundWindow` instability as the blocker.
  - [docs/plans/active/gui_test_platform_exec_plan.md](/C:/dev/sempal/docs/plans/active/gui_test_platform_exec_plan.md#L51) names focus recovery as the open issue.
  - [scripts/run_gui_aiv_suite.ps1](/C:/dev/sempal/scripts/run_gui_aiv_suite.ps1) already categorizes failures, but the repo does not define the pass-rate or case-subset threshold that would be “good enough” for promotion.
- Why this matters: future GUI-test work can keep accumulating without a stable exit criterion.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `docs/plans/active/gui_test_platform_exec_plan.md`
  - `scripts/run_gui_aiv_suite.ps1`
- Risk if guessed incorrectly: either premature CI flakiness or indefinite local-only status for a valuable regression lane.
- Most conservative provisional assumption: keep desktop AIV local-only until one small smoke subset demonstrates repeatable focus-recovery success on the current Windows setup.

## Rejected Ideas

### [-] 1. Split `src/app/controller/playback/transport/selection.rs` immediately

- Why it was considered: [tmp/cleanup_audit_hotspots.md](/C:/dev/sempal/tmp/cleanup_audit_hotspots.md) lists it as the largest live Rust file.
- Why it was rejected: the file appears to hold one cohesive selection-drag/snap/retarget subdomain, and I did not find repository evidence of a current ownership or correctness failure beyond size alone.
- What evidence was missing: a concrete bug, duplicated subdomain, or recurring change-friction signal tied specifically to this file shape.

### [-] 2. Promote the full desktop-AIV suite into default CI now

- Why it was considered: the GUI test platform already exports semantic manifests and categorized desktop-AIV reports.
- Why it was rejected: [docs/gui_test_platform.md](/C:/dev/sempal/docs/gui_test_platform.md#L187) still documents foreground/focus instability as a known blocker.
- What evidence was missing: a stable smoke subset with a defined promotion threshold and repeatable success evidence on the current Windows setup.

### [-] 3. Replace the repository’s custom CLI parsers with `clap`

- Why it was considered: several support binaries parse arguments manually.
- Why it was rejected: the repo explicitly prefers minimal dependencies, and I did not find repository-local parser bugs that justify a framework switch.
- What evidence was missing: concrete maintenance failures or correctness regressions caused by the current parser approach.

### [-] 4. Re-open a broad `app_core` action-semantic redesign

- Why it was considered: action semantics still span catalog, bridge, and controller/history tables.
- Why it was rejected: [src/app_core/actions/tests.rs](/C:/dev/sempal/src/app_core/actions/tests.rs) already provides explicit cross-table guardrails, and I did not find evidence that a larger redesign is currently the highest-ROI move.
- What evidence was missing: active semantic drift that the current guard tests cannot contain.
