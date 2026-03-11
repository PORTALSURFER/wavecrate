# GUI Test Platform

This document describes the layered GUI test platform used to make Sempal's native GUI testable in code and operable by AI-driven desktop automation.

## Goals

- Give every `radiant::app::UiAction` a host-owned catalog entry with explicit coverage requirements.
- Emit a deterministic semantic automation tree from the native shell so code and AI tools can target controls by stable ids instead of screenshots alone.
- Produce machine-readable GUI artifacts that correlate with existing run contracts instead of creating a parallel logging system.
- Support multiple loops:
  - semantic/runtime inner loop
  - projection/snapshot feature loop
  - AIV smoke loop
  - broader manual/nightly desktop loop

## Layer Model

### 1. Host action catalog

The canonical host catalog lives in [src/app_core/actions/mod.rs](/C:/dev/sempal/src/app_core/actions/mod.rs) and [src/app_core/actions/catalog.rs](/C:/dev/sempal/src/app_core/actions/catalog.rs).

What it provides:

- `GuiActionKind`: payload-free identity for every `UiAction` variant
- `GuiActionCatalogEntry`: stable action id, surface, effect class, coverage layers, fixture tags
- `GUI_ACTION_CATALOG`: machine-readable coverage report input
- `action_kind(...)`: exhaustive matcher that breaks compilation when a new `UiAction` is added without catalog support
- coverage guard tests in [src/app_core/actions/tests.rs](/C:/dev/sempal/src/app_core/actions/tests.rs)

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
- sources panel rows and action buttons
- waveform toolbar and waveform interaction region
- browser tabs, search field, rating filters, table rows, and map points
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
- correlates GUI artifacts with run-contract metadata when available
- writes `gui_test_latest.json` after first projection and after each reduced `UiAction`

Artifact contents live in:

- [src/gui_test/artifacts.rs](/C:/dev/sempal/src/gui_test/artifacts.rs)

Each bundle currently includes:

- automation snapshot
- action trace
- projected model summary
- action catalog coverage report
- run id / run manifest path when available

Screenshot fields are present in the schema but currently left empty. AIV remains the screenshot owner in the desktop loop.

### 4. Scenario and CLI layer

The in-process scenario contracts live in:

- [src/gui_test/scenario.rs](/C:/dev/sempal/src/gui_test/scenario.rs)
- [src/gui_test/runner.rs](/C:/dev/sempal/src/gui_test/runner.rs)

The CLI entrypoint lives in:

- [tools/gui-test-cli/src/main.rs](/C:/dev/sempal/tools/gui-test-cli/src/main.rs)

Supported commands:

- `snapshot <output.json>`
- `dispatch-action <action-json> <output.json>`
- `run-scenario <scenario.json> <output.json>`
- `export-aiv-suite <output.json>`

Notes:

- `dispatch-action` accepts serialized `UiAction` JSON directly, so the CLI does not need a second payload parser.
- `run-scenario` uses the same bridge/action path as the catalog and artifact code.
- `export-aiv-suite` currently exports a first-slice suite template with semantic target metadata and observation/screenshot smoke steps.

## Current Dev Loops

### Inner loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1`

This runs:

- catalog/coverage guard tests
- runner smoke tests
- one native-shell runtime hit-test smoke

### Feature loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_suite.ps1`

This runs:

- the contract loop
- a native-shell snapshot fixture smoke
- CLI snapshot export

### AIV smoke loop

- `powershell -ExecutionPolicy Bypass -File scripts/run_gui_aiv_smoke.ps1`

This currently exports the suite template and semantic target metadata that AIV can consume as the desktop lane is expanded.

## Current Gaps

The platform is intentionally first-slice, not final:

- automation coverage is broad but not yet exhaustive for every micro-control
- screenshot capture is still owned by AIV, not the app runtime
- scenario fixtures currently use the default bridge seed rather than a richer fixture library
- the exported AIV suite is a semantic template, not yet a fully semantic click resolver
- no CI gate yet enforces desktop AIV smoke stability

## Immediate Next Steps

1. Expand the automation tree to cover update controls, browser action-strip buttons, and any remaining prompt/options affordances.
2. Add richer seeded GUI fixtures so scenario coverage can exercise realistic browser/source/waveform states.
3. Add more assertions and scenario actions, including targeted node-value and node-action checks.
4. Teach AIV wrappers to consume the exported semantic target metadata directly and resolve node ids to coordinates from `gui_test_latest.json`.
5. Promote the GUI contract loop into `ci_quick` once it is stable enough to be mandatory.
