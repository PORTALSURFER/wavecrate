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

Status: implemented on 2026-03-25

Delivered so far:

- live PowerShell smoke wrapper that launches Sempal in GUI test mode
- semantic node resolution through `gui-test-cli resolve-node-target`
- AIV-driven options/search/tab smoke flow wired to semantic node ids
- focus-recovery retries plus categorized failure reporting for foreground/window/assertion/step failures
- prompt, waveform, browser-map, transport-volume, and update-panel desktop coverage in the `desktop-regression` pack
- per-case manifests/reports/bundles plus top-level `suite-report.json` and `suite-summary.md`

Known limitation:

- foreground activation can still fail on the current Windows setup with `SetForegroundWindow`, so desktop AIV remains local-only and is not yet ready for CI promotion

Acceptance met in this phase:

- focus-recovery handling exists for foreground-sensitive desktop steps
- desktop regression coverage extends beyond options/search into prompt, waveform, browser-map, and update flows
- desktop wrapper exports machine-readable per-case and suite summaries

## Phase 4: CI Promotion

Goal:

- make GUI regression coverage part of normal development gates

Status: partially implemented on 2026-03-25

Delivered so far:

- the semantic GUI contract lane is part of `ci_quick`
- desktop AIV remains intentionally local-only pending a stable promotion bar

Planned work:

1. Define the tiny non-blocking desktop-AIV smoke subset and the evidence threshold for promotion.
2. Add an opt-in PR smoke lane for desktop AIV only after the focus-recovery subset is repeatably stable.
3. Track pass/fail and timing artifacts over time.
