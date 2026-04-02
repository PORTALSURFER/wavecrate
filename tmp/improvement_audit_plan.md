# Improvement Audit Plan

Generated: 2026-04-02
Observed superproject commit: `18616651`
Observed `vendor/radiant` commit: `75b6d980`
Observed workspace state at audit time: dirty worktree with unrelated `src/app/**` edits and a dirty `vendor/radiant` submodule checkout
Status: Phase 2 in progress. Item 1 is implemented and validated locally; remaining items stay in ranked order below.

## Scope

- This plan audits the current live tree only.
- Findings are ranked in strict execution order by expected ROI for the current repository state.
- Recommendations stay inside documented or strongly implied repository intent.
- No implementation was performed during this audit.
- Broad rewrites, speculative features, and preference-only cleanup are excluded.

## Repository Context

- Project purpose: Explicitly documented. `README.md` and `docs/design_principles.md` describe Sempal as an early-alpha Rust desktop tool for triaging, auditioning, editing, and curating local audio samples with a listening-first workflow.
- Maturity level: Explicitly documented. `README.md` warns that the app is early alpha and can modify, rename, or delete user files.
- Primary languages / frameworks / tooling: Explicitly documented. `Cargo.toml` defines a Rust 2024 workspace centered on the root `sempal` crate, companion apps/tools, and the vendored `radiant` GUI/runtime crate.
- Repository shape: Explicitly documented. `docs/ARCHITECTURE.md` splits domain/controller logic across `src/`, migration-facing projections/actions under `src/app_core`, runtime bridge code under `src/gui_runtime`, GUI behavior under `vendor/radiant`, and support tooling under `apps/` and `tools/`.
- Architectural boundaries: Explicitly documented. `README.md`, `docs/ARCHITECTURE.md`, and `docs/gui_test_platform.md` say `src` owns domain state and host policy, while `vendor/radiant` owns GUI behavior and native-shell/runtime mechanics.
- Test strategy: Explicitly documented. `docs/TEST.md` and `.github/workflows/ci.yml` center the repo on `devcheck`, `ci_agent`, `ci_quick`, `ci_local`, deterministic Rust tests, and the Windows GUI contract wrappers.
- Canonical local validation commands: Explicitly documented. Windows flows center on `scripts/run_agent_request.ps1`, `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, `scripts/ci_local.ps1`, and `scripts/run_gui_contract.ps1`.
- Documented priorities: Explicitly documented. `docs/design_principles.md` prioritizes responsiveness, deterministic interaction, reversibility, and non-blocking execution.
- Explicit non-goals: Explicitly documented. `docs/design_principles.md` says Sempal is not a DAW replacement, cloud platform, or retention-driven product.
- What the repo appears to be moving toward: Strongly implied by code/docs. The live tree continues to invest in host-owned GUI contracts, `radiant` native-shell/runtime correctness, and incremental cleanup that preserves strict host/vendor ownership rather than broad rewrites.
- What is merely possible but unsupported: Weakly implied / uncertain. A shared action-id source across host and vendor, promotion of desktop AIV into CI, or large-scale catalog generation beyond the current macro-driven catalog are not yet clearly mandated by repository evidence.

## Audit Baseline

- `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_file_size_budget.ps1 -All` is green on the current tree.
- `powershell -ExecutionPolicy Bypass -File scripts/check_quality_score_drift.ps1` is green and keeps the current score at `4`.
- `powershell -ExecutionPolicy Bypass -File scripts/audit_cleanup_hotspots.ps1` refreshed `tmp/cleanup_audit_hotspots.md`; the snapshot still records oversized Rust files in `vendor/radiant`, `src/app_core/actions/catalog/kinds.rs`, and `src/app/controller/playback/transport/selection.rs`.
- `docs/file_size_budget_allowlist.txt` explicitly says some exceptions should stay centralized unless concrete ownership or correctness pressure emerges. That guidance is treated as repository intent during ranking.

## Ordered Backlog

### 1. [x] Stabilize playback-age filter invalidation in the browser visible-row pipeline

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: S
- Why it matters: the visible-row filtered-stage fingerprint currently includes the current Unix second whenever a playback-age filter is active. That means a user can leave an unchanged browser filter open and still force stage-cache invalidation every second, even though playback-age buckets only change at coarse 7-day and 30-day boundaries.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:26` computes `playback_age_now_unix_secs` on every rebuild.
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:146` includes `(!playback_age_filter.is_empty()).then_some(playback_age_now_unix_secs)` in the filtered fingerprint.
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:216-247` recomputes `playback_age_now_unix_secs` again for the similar-query path.
  - `src/app/state/browser/search.rs:33-45` shows `PlaybackAgeBucket::from_last_played_at(...)` only changes bucket membership at 7-day and 30-day thresholds, not every second.
  - `src/app/controller/library/wavs/browser_pipeline/tests.rs:64-99` only covers the all-rows fast path and one sort mapping; there is no direct coverage for playback-age timing behavior.
- Recommended change: replace the per-second fingerprint input with a stable playback-age invalidation token derived from the next relevant bucket rollover or equivalent coarse boundary. Add direct tests for active playback-age filters and boundary rollover behavior.
- Expected impact: restores the benefit of the staged visible-row cache for age-filtered browsing, makes filter invalidation semantics explicit, and reduces avoidable synchronous work during idle navigation.
- Risks / tradeoffs: long-lived sessions still need deterministic refresh when a sample crosses a bucket boundary; fixing the churn by freezing the filter forever would trade one bug for another.
- Dependencies: none
- Suggested validation:
  - targeted tests in `src/app/controller/library/wavs/browser_pipeline/tests.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed on: `2026-04-02`
- Commit: pending until this item's focused commit is created
- Assumptions used:
  - active playback-age filters should not invalidate every second
  - the conservative refresh contract is explicit data change plus the next relevant age-bucket rollover
- Validation outcome:
  - `cargo test browser_pipeline -- --test-threads=1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Plan-order deviation: none
- Pushed commit: `8b8dd2d1` (`fix(browser): stabilize playback-age filter invalidation`)

### 2. [x] Add direct visible-row pipeline coverage for query, similarity, marked-only, and playback-age branches

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: `visible_rows.rs` owns the main browser staging pipeline, but its direct tests only cover two happy-path cases. That leaves multiple non-trivial branches exposed to silent regressions right where item 1 needs precise behavior locked down.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` contains separate paths for duplicate cleanup, filtered rows, similar-query sorting, query sorting, and filter-only sorting.
  - `src/app/controller/library/wavs/browser_pipeline/tests.rs:64-99` contains only `build_visible_rows_uses_all_fast_path_when_filters_are_idle` and `build_visible_rows_sort_stage_maps_focus_and_loaded_positions`.
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:65-102` has explicit control-flow branches for `duplicate_cleanup`, `similar_query`, text query, and filter-only mode.
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:141-190` and `:216-247` include marked-only, playback-age, and folder-acceptance checks, but no direct tests exercise those combinations.
- Recommended change: add focused controller-level tests that cover at least duplicate-cleanup projection, similar-query filtering, text-query ordering, marked-only filtering, playback-age filtering, and folder-filter acceptance. Keep the fixtures narrow and assert both visible rows and focused/loaded remapping.
- Expected impact: makes future pipeline refactors safer, reduces the risk of regressions in the repository’s core browsing workflow, and gives item 1 a solid behavioral fence.
- Risks / tradeoffs: overusing large end-to-end fixtures could make the tests brittle; the best ROI is in small, table-driven controller fixtures around stage behavior.
- Dependencies: item 1 should land first so the timing contract being tested is the intended one.
- Suggested validation:
  - targeted tests in `src/app/controller/library/wavs/browser_pipeline/tests.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No
- Completed on: `2026-04-02`
- Commit: pending until this item's focused commit is created
- Assumptions used:
  - exact-match filename queries such as `snare` are stable enough in the current fuzzy-matcher contract to anchor one direct query-path test
  - the existing folder-stage acceptance test remains sufficient for folder-map construction while this item closes the remaining direct `build_visible_rows(...)` branch gaps
- Validation outcome:
  - `cargo test browser_pipeline -- --test-threads=1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1` passed
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1` passed
- Plan-order deviation: none

### 3. [ ] Expand automation action-id parity checks from representative nodes to panel-wide contract coverage

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: M
- Why it matters: the repository explicitly treats the host GUI action catalog as canonical, but the native-shell automation snapshot still advertises many `available_actions` through manual string literals and a separate `action_slug` matcher. The current parity tests only sample representative nodes, so a panel can drift to a wrong-but-cataloged action id without failing the default semantic contract suite.
- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical and intentionally host-owned.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98-206` maintains a large manual `action_slug(&UiAction)` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/sidebar.rs:71-110` and `:151-240` emit multiple literal `available_actions` ids directly.
  - `vendor/radiant/src/gui/native_shell/state/automation/browser.rs`, `waveform.rs`, and `dialogs.rs` also hardcode `available_actions` outside the `action_slug` path.
  - `src/gui_test/runner/tests/action_parity.rs:19-101` only checks representative nodes rather than each panel family that emits manual action ids.
- Recommended change: keep the current host/vendor boundary, but add broader fixture-driven parity assertions for each panel family that still hardcodes advertised action ids. Prefer panel- or node-family helpers over one giant snapshot assertion.
- Expected impact: reduces semantic automation drift risk for `gui-test-cli`, semantic runner assertions, and desktop AIV without forcing a larger ownership refactor first.
- Risks / tradeoffs: broader fixture assertions will make intentional action-surface changes noisier; the tests should anchor on stable node families instead of brittle full-tree dumps.
- Dependencies: none
- Suggested validation:
  - targeted `src/gui_test/runner/tests/action_parity.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 4. [ ] Consolidate browser focus and selection ownership across `selection_ops.rs` and `focus_navigation.rs`

- Classification: Architecture improvement
- Confidence: Medium
- ROI: Medium-High
- Effort: M
- Why it matters: browser focus state currently has overlapping mutation paths. Selection helpers already own preview-vs-commit side effects and write `selected_wav` plus last-focused fields, while navigation helpers also write browser-selection state directly before and after calling those helpers. That overlap raises the cost of reasoning about review follow-ups, preview navigation, and future focus-state fixes.
- Evidence:
  - `src/app/controller/library/wavs/selection_ops.rs:113-163` centralizes `select_wav_known_index_with_options(...)`, writes `sample_view.wav.selected_wav`, `last_focused_index`, and `last_focused_path`, and handles preview-vs-commit side effects.
  - `src/app/controller/library/wavs/browser_actions/focus_navigation.rs:153-169` writes `selection_anchor_visible`, `last_focused_index`, and `last_focused_path` directly inside `focus_browser_row_only(...)` before delegating to preview focus helpers.
  - `src/app/controller/library/wavs/browser_actions/focus_navigation.rs:176-199` mutates anchor state again in `commit_focused_browser_row(...)`.
  - `src/app/controller/library/wavs/browser_actions/focus_navigation.rs:322-345` mutates browser focus state again in `follow_browser_review_path(...)`.
  - `src/app/controller/library/wavs/browser_actions/tests.rs` covers several anchor-selection behaviors, but there is no direct test focused on `follow_browser_review_path(...)` or the preview/commit ownership seam.
- Recommended change: pick one clear owner for browser-focus state writes. A conservative path is to keep navigation orchestration in `focus_navigation.rs` while routing concrete focus/selection mutations through one helper layer with explicit preview and commit entrypoints, then add tests around review follow-up and focused-row commit behavior.
- Expected impact: fewer cross-file invariants around browser focus state, lower regression risk in high-frequency navigation, and easier future work on browser review flows.
- Risks / tradeoffs: preview and commit semantics are intentionally different, so centralization must preserve the current lightweight preview path and anchor behavior exactly.
- Dependencies: none
- Suggested validation:
  - targeted tests in `src/app/controller/library/wavs/browser_actions/tests.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

### 5. [ ] Split the `radiant` hotkey catalog into scope-owned binding slices while preserving one flat public contract

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium-High
- Effort: M
- Why it matters: contextual hotkeys are a first-class interaction contract in `docs/design_principles.md`, but one allowlisted `vendor/radiant` file still owns the full binding table, the route resolver, and all tests. That makes overloaded key edits harder to review and increases the chance of cross-scope regressions in a user-visible input surface.
- Evidence:
  - `docs/design_principles.md` says hotkeys are contextual, focus-dependent, and should remain explicit and predictable.
  - `tmp/cleanup_audit_hotspots.md:22` and `:72` report `vendor/radiant/src/app/hotkeys.rs` at `1133` lines during the audit snapshot.
  - `vendor/radiant/src/app/hotkeys.rs:176` defines `HOTKEY_BINDINGS`.
  - `vendor/radiant/src/app/hotkeys.rs:799-838` contains the main resolution logic over the same flat catalog.
  - `vendor/radiant/src/app/hotkeys.rs:857-1081` keeps the tests in the same oversized file.
- Recommended change: keep `HOTKEY_BINDINGS` as the flat exported surface, but build it from smaller scope- or family-owned slices such as `global`, `browser`, `folders`, `sources`, and `waveform`. Add table-driven route assertions for overloaded gesture families and chord precedence.
- Expected impact: smaller diffs for hotkey changes, clearer ownership of contextual keymaps, and lower regression risk in one of the repository’s most visible interaction contracts.
- Risks / tradeoffs: stable ordering matters for overlays and tests, so decomposition has to preserve ordering and exported ids exactly.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test --manifest-path vendor/radiant/Cargo.toml hotkeys -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 6. [ ] Split the oversized `native_vello` runtime gesture test hubs by interaction family

- Classification: Refactor / cleanup
- Confidence: High
- ROI: Medium
- Effort: M
- Why it matters: the largest remaining allowlisted `vendor/radiant` files are runtime interaction tests that mix many unrelated queueing and waveform-gesture cases. They are important correctness coverage, but the current shapes make failures harder to localize and encourage future accumulation in already-large compatibility suites.
- Evidence:
  - `tmp/cleanup_audit_hotspots.md:21`, `:24`, and `:25` report:
    - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` at `1636` lines
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` at `763` lines
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` at `713` lines
  - `docs/file_size_budget_allowlist.txt` still lists all three as explicit legacy exceptions.
  - `tmp/cleanup_audit_hotspots.md:54` also flags a `238`-line gesture-specific test inside `waveform_drag_finish.rs`, which is another sign that multiple gesture families are colliding in one file.
- Recommended change: keep the shared runtime fixtures, but split these suites into focused modules by interaction family: queue input classes, browser scrollbar and row routing, waveform selection creation, waveform finish routing, and modifier-specific gesture behavior.
- Expected impact: preserves the existing runtime coverage while making regressions easier to triage and future gesture additions easier to review.
- Risks / tradeoffs: pure test moves can still make targeted test filters harder to discover if module names are not chosen carefully.
- Dependencies: none
- Suggested validation:
  - targeted `cargo test --manifest-path vendor/radiant/Cargo.toml gui_runtime::native_vello::tests -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Product clarification required: No

### 7. [ ] Revisit `playback/transport/selection.rs` now that the allowlist’s “clearer boundary” condition appears partly satisfied

- Classification: Refactor / cleanup
- Confidence: Medium
- ROI: Medium
- Effort: M
- Why it matters: the file-size allowlist explicitly said to revisit this module only when a clearer seam emerged than the original gesture/playback coupling. The current file now shows a visible split between gesture entrypoints and downstream playback/BPM/transient policy, which makes it a better candidate for behavior-preserving decomposition than when the note was written.
- Evidence:
  - `docs/file_size_budget_allowlist.txt` says `src/app/controller/playback/transport/selection.rs` should be revisited only if a clearer module boundary emerges.
  - `tmp/cleanup_audit_hotspots.md:27` and `:77` report the file at `475` lines during the audit snapshot.
  - `src/app/controller/playback/transport/selection.rs:12-124` centers on gesture lifecycle entrypoints such as `start_selection_drag`, `start_edit_selection_drag`, `update_selection_drag`, `finish_selection_drag`, and related cancel/clear helpers.
  - `src/app/controller/playback/transport/selection.rs:183-371` centers on playback retargeting, transient snapping, BPM scaling, and related helper policy.
  - The same file already keeps local tests close to the behavior contract, which supports conservative extraction.
- Recommended change: keep the public controller-facing entrypoints in `selection.rs`, but extract downstream playback-retarget helpers and snap/BPM policy into focused siblings or internal submodules so the file no longer has to carry both gesture orchestration and downstream transport policy.
- Expected impact: removes one allowlisted production exception and makes waveform selection behavior easier to reason about without changing the public controller surface.
- Risks / tradeoffs: the allowlist note is a warning that this seam used to be too coupled to split safely, so extraction should be conservative and test-led rather than driven by line count alone.
- Dependencies: none
- Suggested validation:
  - targeted tests in `src/app/controller/playback/transport/selection.rs`
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Product clarification required: No

## Open Questions / Missing Definitions

### [!] 1. What is the intended refresh contract for active playback-age filters in long-lived sessions?

- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs:26` and `:146` currently use the current Unix second to invalidate filtered rows.
  - `src/app/state/browser/search.rs:33-45` only changes bucket membership at 7-day and 30-day thresholds.
- Why this matters: item 1 is clearly justified, but the final fix still needs one explicit rule for when an unchanged age-filtered view should refresh in a long-lived session.
- Affected files/modules:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs`
  - `src/app/controller/library/wavs/browser_pipeline/helpers.rs`
  - `src/app/state/browser/search.rs`
- Risk if guessed incorrectly: a fix could either keep the current churn or freeze age-filtered views past their intended rollover boundary.
- Most conservative provisional assumption: do not invalidate every second; refresh when explicit data changes land and when the next relevant age bucket boundary is crossed.

### [!] 2. Should stable action identifiers remain duplicated across the host catalog and the `vendor/radiant` automation layer, or is a shared declaration layer now intended?

- Evidence:
  - `docs/gui_test_platform.md` calls the host action catalog canonical and host-owned.
  - `vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98-206` still maintains a manual `action_slug` matcher.
  - `vendor/radiant/src/gui/native_shell/state/automation/{sidebar,browser,waveform,dialogs}.rs` still include literal `available_actions` ids.
- Why this matters: item 3 can safely strengthen parity tests either way, but any deeper deduplication depends on whether the repository still wants a boundary-preserving duplicate representation or now prefers one declaration source.
- Affected files/modules:
  - `docs/gui_test_platform.md`
  - `src/app_core/actions/catalog/`
  - `src/gui_test/runner/tests/action_parity.rs`
  - `vendor/radiant/src/gui/native_shell/state/automation/`
- Risk if guessed incorrectly: a refactor could either violate the intended host/vendor boundary or leave a now-unwanted duplicate source in place.
- Most conservative provisional assumption: keep the host catalog canonical, preserve the current vendor-owned snapshot emission path, and expand parity tests before attempting any shared-source refactor.

### [!] 3. Which layer should own authoritative browser focus-state writes after preview and review follow-up navigation?

- Evidence:
  - `src/app/controller/library/wavs/selection_ops.rs:113-163` already owns preview-vs-commit side effects and writes `selected_wav`, `last_focused_index`, and `last_focused_path`.
  - `src/app/controller/library/wavs/browser_actions/focus_navigation.rs:153-169`, `:176-199`, and `:322-345` also write related focus and anchor fields directly.
- Why this matters: item 4 is safe only if one layer is treated as orchestration and the other as the owner of the durable focus-state mutations.
- Affected files/modules:
  - `src/app/controller/library/wavs/selection_ops.rs`
  - `src/app/controller/library/wavs/browser_actions/focus_navigation.rs`
  - `src/app/controller/library/wavs/browser_actions/tests.rs`
- Risk if guessed incorrectly: a cleanup could preserve line count goals while still leaving the real ownership ambiguity in place or subtly changing preview-vs-commit behavior.
- Most conservative provisional assumption: centralize browser focus-state writes in one helper layer and leave `focus_navigation.rs` responsible only for choosing the next target and intent.

## Rejected Ideas

### [-] 1. Collapse the host GUI action catalog into one newly generated registry

- Why it was considered: `src/app_core/actions/catalog/kinds.rs` remains large, `src/app_core/actions/catalog/entries.rs` still keeps a large macro table, and `src/app_core/actions/tests.rs` contains several synchronization-style assertions.
- Why it was rejected: the current host catalog is already deliberately canonical and partially centralized through `gui_action_catalog!`, while `docs/gui_test_platform.md` explicitly preserves host ownership and decouples `radiant` from Sempal-specific coverage policy.
- What evidence was missing: a concrete correctness or ownership failure stronger than file size and synchronization tests.

### [-] 2. Split `library/background_jobs/polling/library_handlers.rs` by job family right now

- Why it was considered: the handler file still mixes folder-scan, search, analysis-failure, selection-export, and normalization result paths.
- Why it was rejected: existing tests already cover stale folder-scan handling, stale browser-search rejection, and selection-export history cleanup, so the evidence is not yet strong enough to justify a high-ROI split in this audit pass.
- What evidence was missing: a demonstrated correctness problem, missing coverage on the most failure-prone paths, or a documented ownership boundary that the current file is violating.

### [-] 3. Split `src/selection/range.rs` just because it still sits near the file-size budget

- Why it was considered: `tmp/cleanup_audit_hotspots.md` still reports it near the top of the root-crate size hotspots.
- Why it was rejected: `docs/file_size_budget_allowlist.txt` explicitly says the file should stay together unless a genuinely separate waveform-editing subdomain appears, and the module header reinforces that cohesion.
- What evidence was missing: a concrete ownership or correctness seam stronger than the current “range + fades + gain” domain model.

### [-] 4. Promote desktop AIV coverage into a default CI gate now

- Why it was considered: stronger action-id parity coverage would improve the semantic contract feeding desktop AIV.
- Why it was rejected: `docs/gui_test_platform.md` still says desktop AIV is local-only and not yet stable enough for CI gating.
- What evidence was missing: a repository-defined stability threshold or failure-budget policy for promoting AIV beyond local/manual runs.
