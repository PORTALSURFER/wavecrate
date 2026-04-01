# GUI Test Platform

This document describes the layered GUI test platform used to make Sempal's native GUI testable in code and operable by AI-driven desktop automation.

## Goals

- Give every `radiant::app::UiAction` a host-owned catalog entry with explicit coverage requirements and a clear public-dispatch policy.
- Emit a deterministic semantic automation tree from the native shell so code and AI tools can target controls by stable ids instead of screenshots alone.
- Produce machine-readable GUI artifacts that correlate with existing run contracts instead of creating a parallel logging system.
- Support multiple loops:
  - semantic/runtime inner loop
  - projection/snapshot feature loop
  - AIV smoke loop
  - broader manual/nightly desktop loop

## Layer Model

### 1. Host action catalog

The canonical host catalog lives in [src/app_core/actions/mod.rs](/C:/dev/sempal/src/app_core/actions/mod.rs) and the [src/app_core/actions/catalog/](/C:/dev/sempal/src/app_core/actions/catalog) module tree.

What it provides:

- `GuiActionKind`: payload-free identity for every `UiAction` variant
- `GuiActionCatalogEntry`: stable action id, surface, effect class, coverage layers, fixture tags, and dispatch policy
- `GUI_ACTION_CATALOG`: machine-readable coverage report input
- `action_kind(...)`: exhaustive matcher that breaks compilation when a new `UiAction` is added without catalog support
- coverage guard tests in [src/app_core/actions/tests.rs](/C:/dev/sempal/src/app_core/actions/tests.rs)

Dispatch-policy note:

- most catalog entries are public host-dispatch actions and may be used by `gui-test-cli dispatch-action` and in-process scenarios
- runtime-internal gesture-arm actions are still cataloged for coverage/tracing, but the public GUI runner rejects them explicitly instead of silently dispatching them into an unhandled no-op path

Why it lives in `app_core`:

- coverage policy is a host concern, not a renderer concern
- tools and tests already depend on `sempal`
- the catalog can evolve without coupling `radiant` to Sempal-specific fixture/AIV policy

### 2. Native-shell automation snapshot

The semantic automation tree is emitted from the `radiant` native shell, not inferred from pixels.

Runtime-facing types live in:

- [vendor/radiant/src/app/automation.rs](/C:/dev/sempal/vendor/radiant/src/app/automation.rs)

Native-shell snapshot building lives in:

- [vendor/radiant/src/gui/native_shell/state/automation.rs](/C:/dev/sempal/vendor/radiant/src/gui/native_shell/state/automation.rs)
- [vendor/radiant/src/gui_runtime/native_vello.rs](/C:/dev/sempal/vendor/radiant/src/gui_runtime/native_vello.rs)

Current node coverage includes:

- top bar volume and options controls
- top bar update panel metadata and update action nodes
- sources panel rows and action buttons
- waveform toolbar and waveform interaction region
- browser tabs, search field, rating filters, action buttons, table rows, scrollbars, and map points
- options panel, prompt overlay, and modal progress overlay
- status bar readout

Each node carries:

- stable semantic id
- role
- quantized bounds
- current value/selection/enabled state
- available stable action ids
- extra deterministic metadata

This is the surface AIV should prefer over screenshot matching whenever possible.

### 3. Deterministic GUI test mode

The runtime/host test-mode contract lives in:

- [src/gui_test/config.rs](/C:/dev/sempal/src/gui_test/config.rs)
- [src/app_core/native_bridge/gui_test.rs](/C:/dev/sempal/src/app_core/native_bridge/gui_test.rs)
- [src/main.rs](/C:/dev/sempal/src/main.rs)

Environment variables:

- `SEMPAL_GUI_TEST_MODE=1`
- `SEMPAL_GUI_TEST_ARTIFACT_DIR=<dir>`
- `SEMPAL_GUI_TEST_VIEWPORT=<width>x<height>`
- `SEMPAL_GUI_TEST_SCENARIO=<name>`

Current first-slice behavior:

- fixes the runtime viewport
- disables maximized startup in GUI test mode
- routes named fixture tags through deterministic controller seeds when requested
- correlates GUI artifacts with run-contract metadata when available
- writes `gui_test_latest.json` after first projection and after each reduced `UiAction`

Artifact contents live in:

- [src/gui_test/artifacts.rs](/C:/dev/sempal/src/gui_test/artifacts.rs)

Each bundle currently includes:

- automation snapshot
- action trace with handled/unhandled dispatch status
- projected model summary
- action catalog coverage report
- run id / run manifest path when available

Screenshot fields are present in the schema but currently left empty. AIV remains the screenshot owner in the desktop loop.

### 4. Scenario and CLI layer

The in-process scenario contracts live in:

- [src/gui_test/scenario.rs](/C:/dev/sempal/src/gui_test/scenario.rs)
- [src/gui_test/runner/mod.rs](/C:/dev/sempal/src/gui_test/runner/mod.rs)
- [src/gui_test/runner/assertions.rs](/C:/dev/sempal/src/gui_test/runner/assertions.rs)
- [src/gui_test/runner/bundle.rs](/C:/dev/sempal/src/gui_test/runner/bundle.rs)

The CLI entrypoint lives in:

- [tools/gui-test-cli/src/main.rs](/C:/dev/sempal/tools/gui-test-cli/src/main.rs)

Supported commands:

- `snapshot <output.json>`
- `dispatch-action <action-json> <output.json>`
- `run-scenario <scenario.json> <output.json>`
- `run-scenario-pack <pack-name> <output-dir>`
- `export-aiv-suite <output.json>`
- `export-aiv-suite <pack-name> <output.json>`
- `resolve-node-target <artifact.json> <node-id>`

Notes:

- `dispatch-action` accepts serialized `UiAction` JSON directly, so the CLI does not need a second payload parser.
- scenario steps are intentionally limited to `dispatch_action` and `assert`; snapshot capture is provided by the dedicated `snapshot` command instead of an in-band `capture_snapshot` step.
- `run-scenario` uses the same bridge/action path as the catalog and artifact code.
- `run-scenario-pack` executes named packs over deterministic fixture seeds and writes one artifact per scenario.
- `resolve-node-target` translates one semantic node id into window-relative target geometry for AIV wrappers.
- `export-aiv-suite <output.json>` remains a backward-compatible alias for the `desktop-smoke` desktop-AIV pack.
- `export-aiv-suite <pack-name> <output.json>` exports the typed desktop-AIV manifest consumed by the PowerShell desktop runner.

## Current Dev Loops

### Inner loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`

This runs:

- catalog/coverage guard tests
- runner smoke tests
- one native-shell runtime hit-test smoke

This semantic/runtime contract slice is part of the normal Windows quick gate:

- `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`

### Feature loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`

This runs:

- the contract loop
- a native-shell snapshot fixture smoke
- CLI snapshot export
- the `contract-smoke` scenario pack, including transport play/volume and map-point focus slices

### AIV desktop loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_smoke.ps1`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_suite.ps1 -PackName desktop-regression`

This now launches the real app in GUI test mode per manifest case, relaunches with a fresh sandbox per fixture, consumes semantic node targets from `gui_test_latest.json`, and drives browser, options, prompt, waveform, and update flows through AIV.

The wrapper now retries foreground recovery before focus-sensitive desktop steps and records failure categories (`focus_recovery`, `window_lifecycle`, `app_assertion`, `step_execution`) in the suite report so local failures are easier to triage.

Desktop packs:

- `desktop-smoke`: `startup_ready`, `options_open_close`, `browser_search_type_smoke`
- `desktop-regression`: `startup_ready`, `browser_search_select_commit`, `browser_tabs_and_rating_filters`, `browser_playback_age_filters`, browser scroll/refocus cases, `browser_map_point_focus`, `options_open_close`, `prompt_confirm`, `prompt_cancel`, `waveform_transport_button`, `transport_volume_slider_drag`, `waveform_transport_cursor_selection_zoom`, `waveform_outside_click_clears_both_marks`, `update_panel_actions`

Desktop runner outputs:

- per-case `case-manifest.json`
- per-case `case-report.json`
- per-case `aiv-bundle.json`
- per-case runtime artifacts and screenshots
- top-level `suite-report.json`
- top-level `suite-summary.md`

Known limitation:

- on the current Windows setup, `aiv` can still fail foreground activation with `SetForegroundWindow`; the wrapper now retries and categorizes those failures, but the loop is still local-only and not yet stable enough to promote into CI.

## Current Gaps

The platform is intentionally first-slice, not final:

- automation coverage is broad but not yet exhaustive for every micro-control
- screenshot capture is still owned by AIV, not the app runtime
- desktop AIV stability still depends on Windows foreground/focus behavior even after wrapper-level retries
- no CI gate yet enforces desktop AIV smoke stability

## Immediate Next Steps

1. Keep the semantic GUI contract loop healthy inside `ci_quick` without pulling unstable desktop AIV coverage into the default gate.
2. Collect repeated local evidence from the categorized desktop-AIV reports before considering any stronger promotion.
