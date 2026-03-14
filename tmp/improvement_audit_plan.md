# Evidence-Driven Improvement Audit Plan

Generated: 2026-03-14
Repository: `C:\dev\sempal`
Branch: `next`
Phase: 2 in progress
Implementation status: items 1-3 are complete; items 4-8 remain pending.

## Repository Context

- Project purpose: Rust desktop sample-triage and sample-editing application with a native GUI, playback/editing flows, sample-source scanning/database state, updater support, and semantic GUI automation.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/design_principles.md`, `docs/ARCHITECTURE.md`
- Maturity level: active application with strong local guardrails and growing subsystem decomposition, but still carrying visible file-size debt, doc drift, and a few unresolved boundary questions.
  - Confidence: Strongly implied by code/docs
  - Evidence: `.github/workflows/ci.yml`, `docs/QUALITY_SCORE.md`, `tmp/cleanup_audit_hotspots.md`, `docs/plans/index.md`
- Primary languages/frameworks/tooling: Rust 2024 workspace, Cargo, PowerShell-first workflow wrappers on Windows, `vendor/radiant` for native GUI/runtime integration, and AIV-backed desktop automation support.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `AGENTS.md`, `docs/README.md`, `docs/TEST.md`, `docs/gui_test_platform.md`
- Repository shape: main app in `src/`, companion tools/apps in `apps/` and `tools/`, documentation under `docs/`, temporary working plans under `tmp/`, and the `vendor/radiant` submodule.
  - Confidence: Explicitly documented
  - Evidence: `Cargo.toml`, `docs/ARCHITECTURE.md`, repository tree
- Architectural boundaries: application/domain logic lives in `src/**`, backend-neutral UI/action contracts live in `src/app_core/**`, and native-shell/runtime behavior lives in `vendor/radiant/**`.
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `docs/ARCHITECTURE.md`, `docs/gui_migration_parity.md`
- Test strategy: fast Rust tests first, then broader validation through PowerShell CI wrappers; GUI semantics are treated as contract surfaces and complemented by desktop automation.
  - Confidence: Explicitly documented
  - Evidence: `docs/TEST.md`, `AGENTS.md`, `docs/gui_test_platform.md`
- Canonical local validation commands on Windows:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - Confidence: Explicitly documented
  - Evidence: `README.md`, `AGENTS.md`, `docs/README.md`, `docs/TEST.md`
- Documented priorities: responsiveness, non-blocking workflows, deterministic interaction, explicit documentation, and reversible/safe changes.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`, `AGENTS.md`
- Explicit non-goals: DAW replacement, social/cloud platform scope, and speculative framework rewrites.
  - Confidence: Explicitly documented
  - Evidence: `docs/design_principles.md`

## ROI-Ranked Backlog

### [x] 1. Restore a rustfmt-clean baseline so `scripts/ci_local.ps1` can reach deeper validation

- Classification: Developer-experience improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The documented full local CI path currently stops at formatting drift, so it cannot exercise the deeper lint/test gates it is supposed to represent.
- Evidence:
  - Observed during this audit on 2026-03-14: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` failed at `cargo fmt --all -- --check`.
  - The failure reported formatting drift in `src/app/controller/library/wavs/browser_search_worker/pipeline.rs`, `src/app/controller/playback/tagging/mod.rs`, `src/app_core/app_api.rs`, and `src/app_core/controller.rs`.
  - `README.md`, `docs/README.md`, `docs/TEST.md`, and `AGENTS.md` all describe `scripts/ci_local.ps1` as the canonical local parity command.
- Recommended change:
  - Normalize the current formatting drift and keep the repo at a rustfmt-clean baseline.
  - Treat this as a baseline repair, not a semantic change.
- Expected impact:
  - Restores access to deeper local CI signal.
  - Removes a low-value blocker from every future validation pass.
- Risks / tradeoffs:
  - Low.
  - May reveal the next real baseline blocker immediately after fmt is fixed.
- Dependencies:
  - None.
- Suggested validation:
  - `cargo fmt --all -- --check`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: `075e3a5f` (`fix(ci): restore rustfmt baseline`)
- Validation outcome:
  - `cargo fmt --all -- --check` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` advanced past rustfmt, guardrails, clippy, and rustdoc, then failed later on `vendor/radiant` test `gui::native_shell::layout_adapter::controls::controls_tests::toolbar_search_field_uses_ratio_width_inside_full_host`.
- Assumptions:
  - Reformatting these four files is behavior-preserving because the changes are limited to rustfmt ordering and line-wrapping.

### [x] 2. Define the expected Windows full-CI baseline after the formatting gate

- Classification: Product-definition gap
- Confidence: Medium
- ROI: High
- Effort: S
- Why it matters:
  - The repository documents `scripts/ci_local.ps1` as the canonical local parity command, but current handoff docs also describe a known later failure in `vendor/radiant`. Without an explicit baseline, maintainers cannot tell whether full local CI is expected to be green or merely informative.
- Evidence:
  - `AGENTS.md` currently says broader CI parity is still blocked after the migration-boundary gate by `vendor/radiant` test `gui::native_shell::layout_adapter::controls::controls_tests::toolbar_search_field_uses_ratio_width_inside_full_host`.
  - `MEMORY.md` currently repeats the same known-failure note.
  - `README.md`, `docs/README.md`, and `docs/TEST.md` still present `scripts/ci_local.ps1` as the canonical validation flow without documenting that exception.
- Recommended change:
  - Either fix the known failing `vendor/radiant` test so the documented parity path is actually green, or explicitly document that `ci_local.ps1` is currently expected to fail on that named test and explain the gating rule.
- Expected impact:
  - Removes ambiguity about what “full local parity” means on Windows.
  - Prevents agents and maintainers from treating a known baseline failure as a regression or, worse, ignoring a real new failure.
- Risks / tradeoffs:
  - If the test is flaky or environment-specific, a fix may be more involved than documentation.
  - If only documentation is updated, the repo still carries the actual failing baseline.
- Dependencies:
  - Item 1 should land first so `ci_local.ps1` reliably reaches the later failure.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - If documenting an exception, verify all handoff/docs reference the same expected outcome.
- Product clarification required: Yes
- Completed: 2026-03-14
- Commit: `2250d6fd` in `vendor/radiant` (`fix(layout): square browser toolbar action slot`), `1550b6eb` in `sempal` (`fix(ci): restore windows local parity`)
- Validation outcome:
  - `cargo test -p radiant toolbar_search_field_uses_ratio_width_inside_full_host -- --nocapture` passed.
  - `cargo test -p radiant toolbar_hit_test_ignores_empty_right_host_area -- --nocapture` passed.
  - `cargo nextest run --workspace --all-targets --no-fail-fast --failure-output immediate-final --status-level fail` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1` passed.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1` passed end-to-end.
- Assumptions:
  - The documented expectation for Windows full local parity is that `scripts/ci_local.ps1` should be green, so the highest-value resolution is to fix the exposed `vendor/radiant` baseline defect rather than institutionalize a known failure.

### [x] 3. Repair stale docs and handoff references left behind by recent module splits

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The repo’s orientation docs currently point readers to deleted files and stale audit states. That makes onboarding and follow-up work materially less reliable.
- Evidence:
  - `docs/README.md:27-30` still says `tmp/improvement_audit_plan.md` is Phase 1-only from 2026-03-13.
  - `docs/plans/index.md:15-19` still says the improvement audit execution record has “items 1-14 completed on 2026-03-13”.
  - `docs/gui_test_platform.md:20` still points to nonexistent `src/app_core/actions/catalog.rs`.
  - `docs/ARCHITECTURE.md:38` and `docs/ARCHITECTURE.md:73` still point to nonexistent `src/external_drag.rs`.
- Recommended change:
  - Refresh the stale cross-links and status text so docs point at the live module tree and current audit state.
  - Keep the edits narrowly factual; do not expand scope into new architecture guidance.
- Expected impact:
  - Improves discoverability and reduces false starts for future contributors and agent sessions.
- Risks / tradeoffs:
  - Low.
  - Must avoid quietly changing meaning instead of just correcting references.
- Dependencies:
  - None.
- Suggested validation:
  - Open each referenced path after updating.
  - `rg -n "catalog\\.rs|external_drag\\.rs|items 1-14|2026-03-13|Phase 1 complete" docs`
- Product clarification required: No
- Completed: 2026-03-14
- Commit: pending
- Validation outcome:
  - `rg -n "catalog\\.rs|external_drag\\.rs|items 1-14|2026-03-13|Phase 1 complete" docs` only found the intentionally still-pending file-size planning references and the unchanged cleanup-plan Phase 1 note.
  - Verified live path targets exist for `src/app_core/actions/catalog/` and `src/external_drag/`.
- Assumptions:
  - The recent action-catalog and external-drag splits are complete enough that docs should point at the module directories instead of reintroducing single-file references.

### [ ] 4. Refresh or retire stale file-size debt planning documents so they match the live hotspot scan

- Classification: Documentation gap
- Confidence: High
- ROI: High
- Effort: S
- Why it matters:
  - The repo uses file-size debt documents as planning inputs, but the current debt ledger and active split plan still describe old files that have already been split or deleted.
- Evidence:
  - `docs/file_size_budget_allowlist.txt:23` still lists deleted `src/app_core/actions/catalog.rs`.
  - `docs/file_size_budget_allowlist.txt` also still contains other deleted legacy entries from the recent catalog/test splits.
  - `docs/plans/active/file_size_debt_top5_split_plan.md:11-18` still describes a 2026-02-27 top-5 led by `src/app_core/native_shell.rs`, `src/app/controller/jobs.rs`, and `src/app/controller/library/wavs/browser_search_worker.rs`, all of which have already been split.
  - `tmp/cleanup_audit_hotspots.md` now shows a different live top set led by `src/updater/mod.rs`, `src/app_core/native_bridge/projection_cache.rs`, `src/updater/archive.rs`, and `src/app/controller/tests/playback_loop.rs`.
- Recommended change:
  - Bring `docs/file_size_budget_allowlist.txt` back in sync with the current tree.
  - Either refresh `docs/plans/active/file_size_debt_top5_split_plan.md` to the current hotspots or explicitly retire it in favor of `tmp/cleanup_audit_hotspots.md`.
- Expected impact:
  - Makes cleanup planning trustworthy again.
  - Prevents future work from targeting already-completed splits.
- Risks / tradeoffs:
  - Low.
  - Retiring the old plan without naming a replacement would create another discoverability gap.
- Dependencies:
  - None.
- Suggested validation:
  - `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All`
- Product clarification required: No

### [ ] 5. Split the updater runtime boundary into smaller focused modules before the next updater change

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The updater path is security-sensitive and currently concentrates public API, path/symlink validation, release selection, test hooks, archive download, checksum verification, and extraction behavior across two still-large files.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/updater/mod.rs` at 479 lines and `src/updater/archive.rs` at 469 lines.
  - `src/updater/mod.rs:122-153` exposes the main updater entrypoints, while `src/updater/mod.rs:221-362` also owns path/symlink sanitization and test-only environment hooks.
  - `src/updater/archive.rs:45-282` mixes download, checksum parsing, signature verification, retry logic, and zip extraction limits in one file.
- Recommended change:
  - Separate the updater facade, path-validation helpers, release-asset verification, and archive extraction into focused modules with stable entrypoints.
  - Keep behavior identical and preserve the existing verification tests.
- Expected impact:
  - Lowers review risk in a sensitive code path.
  - Makes future updater/security fixes more local and easier to validate.
- Risks / tradeoffs:
  - Moderate refactor risk if module boundaries are over-designed.
  - Avoid changing updater behavior while splitting.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted updater tests.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 6. Decompose `app_core` projection-cache state and probe logic into narrower ownership layers

- Classification: Architecture improvement
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters:
  - The projection cache is a core bridge between controller state and native projections. It still mixes probe metrics, cache keys, derived-state assembly, and retained cache state in one file, which increases regression risk in a migration-critical boundary.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app_core/native_bridge/projection_cache.rs` at 474 lines.
  - `tmp/cleanup_audit_hotspots.md` identifies `record_lookup` in that file as a 255-line hotspot.
  - `src/app_core/native_bridge/projection_cache.rs:32-290` combines lookup metrics, probe measurements, multiple cache-key structs, and derived-state assembly before the retained cache type itself.
- Recommended change:
  - Split probe/metrics types, projection-key building, and retained cache state into focused modules under `src/app_core/native_bridge/projection_cache/`.
  - Keep the public/native-bridge contract unchanged.
- Expected impact:
  - Makes a migration-critical boundary easier to reason about.
  - Reduces blast radius when projection invalidation or cache-key logic changes.
- Risks / tradeoffs:
  - Moderate, because this code sits on hot UI projection paths.
  - Must avoid adding abstraction layers that hide cache invalidation rules.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted `app_core::native_bridge` tests.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### [ ] 7. Clarify and reduce the dual source-of-truth risk in browser search construction

- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters:
  - The browser still has separate sync and async visible-row builders, and the runtime default differs from the test default. Recent parity work reduced one ordering mismatch, but the codebase still has two behavior-owning paths with unclear long-term ownership.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs:67-381` still owns retained sync visible-row construction.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs:258-398` still owns a separate worker-side visible-row builder.
  - `src/app/controller/library/wavs/browser_search.rs:409-441` still defaults async search to `true` at runtime and `false` under test unless explicitly overridden.
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/parity_tests.rs` shows parity is now tested, which confirms the repo already treats these paths as susceptible to drift.
- Recommended change:
  - First document the intended long-term source of truth for query/filter/sort semantics.
  - Then either keep parity enforcement explicit and local, or extract a smaller shared semantics layer that both paths consume.
- Expected impact:
  - Reduces future behavior drift in a central browsing path.
  - Makes later browser changes safer because maintainers know which layer is authoritative.
- Risks / tradeoffs:
  - Clarification is required before larger refactoring.
  - Forcing a single abstraction too early could hurt clarity or performance.
- Dependencies:
  - None.
- Suggested validation:
  - Existing parity tests plus targeted controller async-path tests.
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: Yes

### [ ] 8. Split the remaining large controller regression catalogs by behavior family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: S
- Why it matters:
  - The repo has already been splitting oversized regression catalogs into behavior-focused module trees. Two large controller test files still combine multiple distinct behaviors, which makes review and navigation harder than it needs to be.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md` lists `src/app/controller/tests/playback_loop.rs` at 468 lines and `src/app/controller/tests/waveform_nav_cursor.rs` at 418 lines.
  - `src/app/controller/tests/playback_loop.rs` combines loop enablement, drag retargeting, BPM stretch, autoplay, and loop-lock behavior.
  - `src/app/controller/tests/waveform_nav_cursor.rs` combines zoom batching, playhead completion, replay-from-start, and cursor marker behavior.
- Recommended change:
  - Split each file into small behavior-grouped submodules while preserving existing assertions and fixture helpers.
- Expected impact:
  - Improves discoverability and lowers review friction in high-value regression coverage.
  - Aligns the remaining test layout with the repo’s already-adopted test-module pattern.
- Risks / tradeoffs:
  - Low.
  - Avoid renaming tests unnecessarily or moving shared fixture helpers into unclear locations.
- Dependencies:
  - None.
- Suggested validation:
  - Targeted controller test runs for the moved modules.
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] What does “Windows full CI parity” currently mean in practice?

- Evidence:
  - `README.md`, `docs/README.md`, and `docs/TEST.md` present `scripts/ci_local.ps1` as the canonical local parity flow.
  - `AGENTS.md` and `MEMORY.md` both record a known later failure in `vendor/radiant`.
- Why this matters:
  - The answer changes whether a red `ci_local.ps1` run is treated as a regression, an accepted exception, or a required fix before other work.
- Affected files/modules:
  - `scripts/ci_local.ps1`, `vendor/radiant`, `README.md`, `docs/README.md`, `docs/TEST.md`, `AGENTS.md`, `MEMORY.md`
- Risk if guessed incorrectly:
  - Agents may either waste time fixing an accepted baseline issue or ignore a real parity regression.
- Most conservative provisional assumption:
  - Treat `ci_local.ps1` as intended to be green, but do not silently assume the known `vendor/radiant` failure is acceptable without an explicit doc note.

### [!] Which browser-search layer is intended to own canonical query/filter/sort semantics?

- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs`
  - `src/app/controller/library/wavs/browser_search.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/parity_tests.rs`
- Why this matters:
  - The right fix differs depending on whether the repo wants shared semantics, explicit parity-only duplication, or a future async-first source of truth.
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_pipeline.rs`
  - `src/app/controller/library/wavs/browser_search.rs`
  - `src/app/controller/library/wavs/browser_search_worker/pipeline/**`
- Risk if guessed incorrectly:
  - A cleanup could add abstraction or remove separation that the runtime architecture actually depends on.
- Most conservative provisional assumption:
  - Preserve the two-path structure for now and only make the authoritative behavior contract explicit before further consolidation.

### [!] Should the old top-5 file-size split plan stay active, or should the hotspot snapshot replace it?

- Evidence:
  - `docs/plans/active/file_size_debt_top5_split_plan.md` still reflects a 2026-02-27 top-5 list that no longer matches the current tree.
  - `tmp/cleanup_audit_hotspots.md` is the newer live snapshot and now points at different hotspots.
- Why this matters:
  - Future cleanup work needs one current, discoverable planning artifact instead of two contradictory ones.
- Affected files/modules:
  - `docs/plans/active/file_size_debt_top5_split_plan.md`
  - `tmp/cleanup_audit_hotspots.md`
  - `docs/plans/index.md`
- Risk if guessed incorrectly:
  - Maintainers may resume from a stale split queue and duplicate already-completed work.
- Most conservative provisional assumption:
  - Keep the old plan only if it is explicitly refreshed; otherwise retire it in favor of the live hotspot snapshot.

## Rejected Ideas

### [-] Rewrite browser search around one async-only pipeline

- Why it was considered:
  - The sync/async browser duplication is visible in the current codebase.
- Why it was rejected:
  - The repo clearly values responsiveness and retained state, but it does not currently document an async-only destination architecture.
- Missing evidence:
  - No ADR, roadmap note, or code comment says the retained sync pipeline should be removed outright.

### [-] Add configurable user keybindings

- Why it was considered:
  - `src/app/controller/ui/hotkeys/actions.rs` is a large catalog.
- Why it was rejected:
  - The large file alone is not evidence that user-configurable hotkeys are an intended product direction.
- Missing evidence:
  - No docs, TODOs, or tests suggest customizable keymaps are planned.

### [-] Propose a broad dependency/toolchain upgrade lane

- Why it was considered:
  - CI and docs drift often correlate with outdated tooling.
- Why it was rejected:
  - This audit found concrete repository-specific issues that are higher leverage, while no current doc/config evidence points to a dependency-upgrade priority.
- Missing evidence:
  - No in-repo roadmap, TODO cluster, or failing compatibility note justifies a broad upgrade initiative right now.
