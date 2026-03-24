# Waveform Click Playback Audit Plan

Last updated: 2026-03-24
Scope: waveform plain-click playback from native pointer routing through host transport start
Status: Phase 2 completed on 2026-03-24.

## Context Summary

- Project purpose: desktop sample browser/editor with a native waveform/transport UI.
  - Classification: Strongly implied by code/docs.
  - Evidence: `docs/README.md`, `docs/gui_test_platform.md`, `src/app/controller/playback/`, `vendor/radiant/src/gui_runtime/native_vello/`.
- Primary implementation boundary for this bug:
  - `vendor/radiant` owns pointer routing and runtime action emission.
  - `src/app_core/native_bridge` owns queued/immediate native action reduction.
  - `src/app/controller/playback` owns transport semantics and actual playback start.
  - Classification: Explicitly documented.
  - Evidence: `docs/gui_test_platform.md`, `src/app_core/controller.rs`, `vendor/radiant/src/app/actions/mod.rs`.
- Canonical local validation commands for this lane:
  - `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - Classification: Explicitly documented.
  - Evidence: `docs/README.md`, `docs/TEST.md`, `AGENTS.md`.

## Ordered Backlog

### [x] 1. Replace the click-release transport action with a cursor-first playback intent

- Classification: Bug fix
- Confidence: High
- ROI: Very high
- Effort: Medium
- Why it matters:
  - The current click-release path in `radiant` emits `SetWaveformCursorPrecise` and then `PlayFromCurrentPlayhead`, but the host controller defines `PlayFromCurrentPlayhead` as "restart from visible playhead first, then cursor, then last marker." That means a stopped waveform click can still restart from the stale playhead instead of the clicked point.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/finish.rs:130` emits `SetWaveformCursorPrecise`.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/finish.rs:132` emits `PlayFromCurrentPlayhead`.
  - `src/app/controller/playback/transport/playback.rs:24` routes `play_from_current_playhead`.
  - `src/app/controller/playback/transport/playback.rs:135` computes `current_playhead_position(...)`.
  - `src/app/controller/playback/transport/playback.rs:139-147` prefer visible playhead over cursor.
- Recommended change:
  - Introduce a native/runtime action whose contract is explicitly "play from cursor/click target" and dispatch that from plain waveform clicks.
  - Wire the host controller to `play_from_cursor()` instead of `play_from_current_playhead()` for that action.
  - Keep `PlayFromCurrentPlayhead` reserved for toolbar/hotkey/global transport semantics.
- Expected impact:
  - Aligns runtime click behavior with the user-visible requirement: click anywhere in the waveform and playback starts there.
  - Removes the stale-playhead ambiguity that current patches keep tripping over.
- Risks / tradeoffs:
  - Requires action-catalog churn across `radiant`, `app_core`, GUI catalog metadata, and tests.
  - Existing callers that intentionally rely on `PlayFromCurrentPlayhead` must stay mapped to the old behavior.
- Dependencies:
  - None.
- Suggested validation:
  - Add controller-level tests proving the new action starts from cursor even when a visible playhead exists elsewhere.
  - Add `radiant` runtime-input tests that assert the new action rather than inferring behavior from a fake bridge.
  - Run `scripts/devcheck.ps1` and `scripts/ci_agent.ps1`.
- Product clarification required:
  - No. The user explicitly requested that clicking anywhere in the waveform should play from that location.
- Completion notes (2026-03-24):
  - Added `PlayFromWaveformCursor` to the shared `UiAction` surface and routed plain waveform click-release through `SetWaveformCursorPrecise + PlayFromWaveformCursor`.
  - The host now dispatches `PlayFromWaveformCursor` to `controller.play_from_cursor()` while preserving `PlayFromCurrentPlayhead` for toolbar/hotkey transport semantics.
  - Commit metadata:
    - `vendor/radiant`: `9af92d07` (`Add cursor-first waveform click playback`)
    - `sempal`: `c640458f` (`Route waveform clicks through cursor playback`)
  - Validation:
    - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
    - focused controller and radiant click-play regressions
  - Assumptions:
    - Plain waveform click should start playback from the clicked cursor target even when a stale visible playhead exists.

### [x] 2. Collapse the fragile “press starts selection, release retroactively means playback” click contract

- Classification: Architecture improvement
- Confidence: High
- ROI: High
- Effort: Medium
- Why it matters:
  - Plain left-click currently routes as `BeginWaveformSelectionAt` on press, then a second code path later reinterprets the interaction as click-to-play if drag slop was not exceeded. That indirection makes the behavior easy to break and hard to reason about across selection-clear, drag-arm, and transport-start changes.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing/mod.rs:86` defaults plain click to `BeginWaveformSelectionAt`.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/drag.rs:221` stores a `WaveformClickSeekPress` side-channel only for selection-arm and clear actions.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/finish.rs:78-84` later decides whether the same gesture should instead become click playback.
  - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs:144` still encodes the press-time contract as selection anchor preservation.
- Recommended change:
  - Make plain waveform click intent explicit in the runtime surface instead of reconstructing it from deferred release heuristics.
  - Keep drag-selection behavior, but model it as an explicit transition from a click-play arm into a drag-selection path when slop is exceeded.
- Expected impact:
  - Simplifies the pointer state machine and reduces hidden coupling between clear behavior, drag arming, and transport semantics.
  - Makes future click behavior changes safer to implement and easier to test.
- Risks / tradeoffs:
  - Input refactor touches hot code and could regress selection creation if not backed by targeted tests.
- Dependencies:
  - Item 1 should define the explicit click-play action first.
- Suggested validation:
  - Add state-machine tests covering plain click, tiny wobble, selection drag, click outside play selection, and click outside both selection types.
  - Run focused `vendor/radiant` pointer tests, then `scripts/ci_agent.ps1`.
- Product clarification required:
  - No for the plain-click contract.
  - Yes only if maintainers want a different behavior while transport is already running.
- Completion notes (2026-03-24):
  - `BeginWaveformSelectionAt` is now arm-only on press in the native runtime; the controller no longer mutates selection state until drag slop is exceeded.
  - This item intentionally shipped in the same radiant commit as item 1 because both changes touch the same click-release/input-state boundary and are not safely separable by file.
  - Commit metadata:
    - `vendor/radiant`: `9af92d07` (`Add cursor-first waveform click playback`)
  - Validation:
    - targeted `vendor/radiant` click/arm tests
    - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - Assumptions:
    - Plain click should remain selection-capable only after actual drag motion, not on press alone.

### [x] 3. Replace the false-positive fake-bridge coverage with controller-backed click-play integration tests

- Classification: Test gap
- Confidence: High
- ROI: High
- Effort: Medium
- Why it matters:
  - The current `radiant` regression tests passed while the real app still failed because the fake bridge models `PlayFromCurrentPlayhead` as “set `transport_running = true`” and never exercises playhead-versus-cursor precedence.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs:336-337` handles `PlayFromCurrentPlayhead` by flipping `transport_running = true`.
  - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs:359-360` repeats the same simplification in the queued bridge path.
  - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs:812` is the current click-release queue regression that still passed while the user reported failure.
- Recommended change:
  - Add at least one cross-boundary test that runs a real `SempalNativeBridge` or an equivalent host-backed reducer/controller path for waveform click release.
  - Assert start position, last start marker, and playhead/cursor state, not just action traces.
- Expected impact:
  - Prevents future “green tests, broken app” loops on waveform transport behavior.
- Risks / tradeoffs:
  - Cross-repo integration tests are slower and may need careful fixture setup.
- Dependencies:
  - None, though Item 1 gives the cleanest final assertion surface.
- Suggested validation:
  - Add one stopped-transport and one visible-stale-playhead scenario.
  - Keep existing fast `radiant` unit tests, but add one host-backed regression in `sempal`.
- Product clarification required:
  - No.
- Completion notes (2026-03-24):
  - Added a controller-backed regression proving `PlayFromWaveformCursor` prefers the cursor over a stale visible playhead.
  - Added bridge invalidation coverage so the cursor-first playback action dirties the transport graph like the other transport-start actions.
  - Commit metadata:
    - `sempal`: `c640458f` (`Route waveform clicks through cursor playback`)
  - Validation:
    - focused controller/native-bridge regressions
    - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - Assumptions:
    - Transport-start coverage is more valuable here than adding another fake-bridge action-trace regression.

### [x] 4. Update the GUI/AIV contract so click-to-play is asserted as playback behavior, not seek-only tracing

- Classification: Test gap
- Confidence: High
- ROI: Medium
- Effort: Small
- Why it matters:
  - The desktop/AIV layer still documents waveform click as `seek_waveform_precise`, which reflects an older contract and does not assert that playback actually starts from the clicked point.
- Evidence:
  - `src/gui_test/aiv/packs/cases/waveform.rs:36` defines `waveform_click_seek_case`.
  - `src/gui_test/aiv/packs/cases/waveform.rs:46` asserts `seek_waveform_precise`.
  - `src/gui_test/aiv/packs/cases/waveform.rs:51` repeats the same expected action.
  - `src/gui_test/aiv/packs.rs:64` still includes that stale case in the exported pack set.
- Recommended change:
  - Replace the seek-only case with a click-play case that asserts the click-specific playback action and, where possible, visible transport/playhead state.
  - Add one case for clicking outside a play selection and one for clicking with no selection.
- Expected impact:
  - Keeps the semantic desktop contract aligned with the actual user-facing behavior.
- Risks / tradeoffs:
  - The current Windows AIV lane is locally flaky, so this should remain supplemental until the app-level contract is stable.
- Dependencies:
  - Item 1 if a new explicit click-play action is added.
- Suggested validation:
  - Run `scripts/run_gui_contract.ps1` and, when the local AIV environment is healthy, rerun the waveform pack case.
- Product clarification required:
  - No.
- Completion notes (2026-03-24):
  - Replaced the stale `waveform_click_seek` desktop contract case with `waveform_click_play`.
  - The semantic assertion now expects `play_from_waveform_cursor` instead of `seek_waveform_precise`, and the regression pack list/case exports were kept in sync.
  - Commit metadata:
    - `sempal`: `c640458f` (`Route waveform clicks through cursor playback`)
  - Validation:
    - `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`
    - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - Assumptions:
    - The semantic desktop contract should follow the new click-play action id even while live AIV execution remains locally flaky.

## Open Questions / Missing Definitions

### [!] Should plain waveform click while playback is already running retarget smoothly or restart from the clicked point?

- Evidence:
  - `src/app/controller/playback/transport/seek.rs` treats seek as a separate deferred/immediate path depending on current playback state.
  - `src/app/controller/playback/transport/playback.rs` separates replay/start semantics from seek semantics.
- Why it matters:
  - The implementation shape differs depending on whether a running transport should continue seamlessly or perform an audible restart at the click target.
- Affected files/modules:
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_input/finish.rs`
  - `src/app_core/controller.rs`
  - `src/app/controller/playback/transport/*.rs`
- Risk if guessed incorrectly:
  - Could introduce discontinuities during live playback or preserve stale semantics that users experience as broken.
- Most conservative provisional assumption:
  - Plain click should use the same “play from clicked point” behavior in both stopped and running states unless maintainers explicitly want smooth retargeting while already playing.

## Rejected Ideas

### [-] Keep patching queue flush timing around `PlayFromCurrentPlayhead`

- Why it was considered:
  - Earlier failures looked like delayed queued waveform actions.
- Why it was rejected:
  - The current evidence shows a semantic mismatch, not just a timing issue: the runtime emits the wrong transport intent for click-to-play.
- Missing evidence:
  - No repository evidence shows that more queue flushing would make `PlayFromCurrentPlayhead` honor the newly clicked cursor when a visible playhead already exists.

## Handoff Notes

- Plan path: `tmp/waveform_click_playback_audit_plan.md`
- Repo-wide handoff files were not updated as part of this scoped Phase 1 audit because the standing repository execution lane did not change, and `MEMORY.md` already had unrelated local edits.
