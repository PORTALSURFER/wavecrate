# GUI Test Platform Execution Plan

## Objective

Build a layered GUI test platform that makes native GUI actions cataloged, semantically testable, snapshot-observable, and consumable by AI desktop tooling.

## Phase 1: Contract Slice

Status: implemented on 2026-03-11

Delivered:

- host-side GUI action catalog with coverage metadata
- coverage guard tests
- native-shell automation snapshot types and builder
- runtime test-mode artifact emission through the bridge
- in-process GUI scenario runner
- `gui-test-cli`
- PowerShell wrapper scripts
- architecture/test documentation

Acceptance met in this phase:

- every `UiAction` is cataloged
- machine-readable action coverage report exists in GUI artifact bundles
- deterministic automation snapshot exists without launching the desktop host
- real app can emit semantic GUI artifacts in GUI test mode

## Phase 2: Fixture Expansion

Goal:

- add seeded GUI fixtures for realistic browser/source/waveform/update states
- add more scenario assertions and reusable scenario packs
- cover high-value workflows with in-process scenarios

Planned work:

1. Introduce named GUI fixtures instead of only the default bridge seed.
2. Add scenario packs for browser search/select/commit, waveform seek/zoom/selection, options open/close, prompt confirm/cancel.
3. Extend contract scripts to run those packs by default.

## Phase 3: Desktop AIV Integration

Goal:

- consume semantic automation artifacts from the live app while AIV drives the desktop

Planned work:

1. Add wrapper scripts that launch Sempal with `SEMPAL_GUI_TEST_MODE=1`.
2. Resolve semantic node ids from `gui_test_latest.json` into window-relative targets for AIV.
3. Replace coordinate-heavy AIV smoke flows with semantic target resolution.

## Phase 4: CI Promotion

Goal:

- make GUI regression coverage part of normal development gates

Planned work:

1. Promote the GUI contract lane into `ci_quick`.
2. Add an opt-in PR smoke lane for AIV once stable.
3. Track pass/fail and timing artifacts over time.
