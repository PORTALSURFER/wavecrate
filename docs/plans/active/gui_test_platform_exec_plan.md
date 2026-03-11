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

Status: implemented on 2026-03-11

Delivered:

- named GUI fixtures: `browser`, `waveform`, `options`, `prompt`, `update`
- controller-seeded fixture bridge wrapper reused by the CLI runner and real desktop runtime
- reusable `contract-smoke` scenario pack
- richer semantic assertions including node absence, enabled state, action availability, and metadata contains
- contract/suite scripts updated to run the broader GUI pack by default

## Phase 3: Desktop AIV Integration

Status: in progress on 2026-03-11

Delivered so far:

- live PowerShell smoke wrapper that launches Sempal in GUI test mode
- semantic node resolution through `gui-test-cli resolve-node-target`
- AIV-driven options/search/tab smoke flow wired to semantic node ids

Open issue:

- foreground activation can fail on the current Windows setup with `SetForegroundWindow`, so the wrapper is not yet stable enough for CI promotion

Remaining work:

1. Add focus-recovery or click-fallback handling for AIV foreground failures.
2. Expand the smoke wrapper to cover waveform and prompt interactions once focus is reliable.
3. Export machine-readable pass/fail summaries from the AIV wrapper.

## Phase 4: CI Promotion

Goal:

- make GUI regression coverage part of normal development gates

Planned work:

1. Promote the GUI contract lane into `ci_quick`.
2. Add an opt-in PR smoke lane for AIV once stable.
3. Track pass/fail and timing artifacts over time.
