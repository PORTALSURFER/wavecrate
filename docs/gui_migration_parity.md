# GUI migration parity matrix (`egui` -> `radiant`)

This document tracks feature parity work after cutting over the main Sempal UI
to `radiant` (`native_vello`) as the only runtime path.

## P0 (main runtime cutover)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Runtime startup | Native backend as only main path | Done | Sempal |
| Sources panel | Select source row + reflect missing state | Done (bridge projection) | Sempal + Radiant |
| Browser panel | Row focus and multi-select (click/ctrl/shift patterns) | Done (controller wired) | Sempal |
| Browser panel | Search query update and busy indicator | Done (bridge projection + action) | Sempal |
| Waveform | Seek/cursor/selection/zoom + loop toggle | Done (bridge action mapping) | Sempal |
| Transport | Play/pause toggle | Done | Sempal |
| Undo stack | Undo/redo shortcuts from native runtime | Done | Sempal |
| Status surface | Status text visible in native shell | Done (single-line footer text) | Radiant |
| Focus model | Focus browser/sources/waveform/search targets | Done | Sempal |

## P1 (close parity after main-path cutover)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Browser actions | Context menus (rename/tag/delete) | Done (native action strip + bridge routing) | Radiant + Sempal |
| Source management | Folder actions (rename/create/delete/recovery) | Done (native prompt/action flows + validation/error gating + compact recovery polish) | Radiant + Sempal |
| Workflow overlays | Progress, drag overlays, prompts | Done (native overlay rendering + prompt/progress actions) | Radiant |
| Update UX | In-app release notes/update prompts | Done (native top-bar now consumes projected status/hint/release metadata labels with check/open/install/dismiss routing) | Radiant + Sempal |
| Map view | Cluster map interactions and rendering | Done (native map tab now renders normalized points with click-to-focus routing plus projected legend/selection/hover/cluster/viewport labels) | Radiant + Sempal |

## P2 (post-cutover polish and expansion)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Rendering polish | Motion/styling refinement inspired by Xilem/Vello | Done (map/update header chrome now keeps metadata inside tokenized bands while preserving projected copy hierarchy, with compact footer metadata fallbacks for dense viewports) | Radiant |
| Layout contract | Tokenized header/body/footer geometry shared by paint + hit testing | Done (top-bar cluster reserve assertions and single-line browser-header metadata capacity checks now enforce tokenized non-overlap contracts across compact/standard/wide tiers) | Radiant |
| Sidebar layout | Tokenized source/folder section sizing and action controls | Done (tiered sizing and compact edge-case guards) | Radiant |
| Scale behavior | Browser virtualization/perf tuning beyond 48 rendered rows | Done (5k-row virtualization now validates focus/tail preservation and deterministic frame rebuilds across compact/standard/wide tiers) | Radiant |
| Tooling | Snapshot + interaction golden tests for native shell | Done (tiered contract tests now include map/update header-band fit checks plus 5k-row deterministic virtualization assertions) | Radiant + Sempal |

## Migration notes

- New backend-neutral projection helpers now live under `src/app_core`.
- Native bridge orchestration now lives in `src/app_core/native_bridge.rs`.
- Main runtime backend selection has been removed; `src/main.rs` now boots native Vello directly.
- Native shell layout now derives panel/frame metrics from shared style tokens (`vendor/radiant/src/gui/native_shell/style.rs`) and exposes explicit panel bands in `vendor/radiant/src/gui/native_shell/layout.rs`.
- Browser region migration has started from triage columns toward the classic table shell:
  tabs + toolbar + table header/rows/footer are now explicit layout bands used by paint and hit-testing.
- Browser chrome now renders explicit search + state/sort chips and tokenized table columns (`#`, `Sample`, `Bucket`) instead of a single placeholder toolbar/header text line.
- Top bar now uses a tokenized split layout (title row + controls row) with explicit options/volume meter geometry instead of hardcoded text offsets.
- Projection now supplies native render hints for browser tab/sort/search labels and waveform tempo/zoom labels to avoid hardcoded placeholder copy.
- Browser rows now support explicit bucket labels in native projections (for example BPM badges) instead of relying only on coarse column tags.
- Native browser map tab now consumes projected point clouds and emits click-to-focus sample actions.
- Native top bar now consumes update-check projection state and emits update actions (check/open/install/dismiss).
- Native map/update projection now also carries explicit chrome labels
  (`legend/selection/viewport` for map and `status/action-hint` for updates)
  so renderer copy no longer needs update/map-specific fallback strings.
- Native map projection now also carries explicit `hover` and `cluster` labels,
  and update projection now carries explicit release metadata labels, so
  map/update chrome can remain projection-driven without renderer-local strings.
- Native browser/waveform chrome text now comes from projected host models
  (`BrowserChromeModel`, `WaveformChromeModel`) instead of renderer-local hardcoded labels.
- Baseline geometry/copy target for legacy parity is documented in
  `docs/native_shell_legacy_baseline.md`.
- Native layout guard rails now come from sizing tokens (viewport clamp, top-bar split,
  waveform/browser minimum split, browser footer/tabs/header minima) and are validated through
  `ShellLayout::contract_snapshot(...)` assertions in native-shell tests.
- Native browser table now uses higher-contrast alternating row striping and refined bucket-chip
  blend levels to better match classic list readability, and waveform title text uses primary
  hierarchy emphasis instead of muted metadata styling.
- Native map/update header metadata now uses single-band compact composition so projected labels
  remain inside tokenized header/control rows in dense viewports, with hover/cluster/viewport
  details consolidated into footer metadata lines.
- Native layout tiering now keeps the classic-dense baseline as default up through common
  desktop widths (wide tier starts above 2100 logical px), and wide-tier row/header metrics
  remain table-dense instead of switching to oversized card spacing.
- Browser projection/render windows now target larger native-shell stress sets
  (`MAX_RENDERED_BROWSER_ROWS` 512 in host projection and higher per-tier native row caps),
  with explicit tests for tail clamping, focus preservation, and deterministic large-dataset frames.
- Installer/updater binaries now run on the native radiant host path (`run_native_vello_app`);
  dependency cleanup for legacy `app` modules remains a follow-up task.
- App-core native-shell projection and native bridge prompt/tab routing now consume
  migration-facing `app_core::{controller,state}` aliases instead of direct
  `app::state` paths in host integration code.
- The migration-facing `app_core::controller::AppController` alias now points to
  `app::controller::AppController` to keep `EguiController` naming
  confined to legacy app internals.
- `app_core::state` now owns migration-facing enums for browser tab, triage
  column, update status, and map render mode, with explicit conversion bridges
  to legacy `app::state` enums at runtime boundaries, plus a migration-facing
  `UiState` alias for projection internals.
- `app_core::state` now routes legacy state conversions through a single
  `legacy_state` module alias, reducing repeated direct legacy module paths in
  migration-facing state glue.
- `app_core::{controller,view_model}` now route legacy references through
  module aliases (`legacy_controller`, `legacy_view_model`) so migration-facing
  glue no longer repeats direct legacy module paths.
- `app_core::native_shell` now consumes migration-facing state aliases directly
  (`SampleBrowserActionPrompt`, `FolderActionPrompt`, `DestructiveEditPrompt`,
  `DragTarget`) in projection glue instead of fully-qualified state paths.
- `app_core::actions` now owns migration-facing native runtime aliases
  (`NativeUiAction`, `NativeAppModel`, `NativeFrameBuildResult`,
  `NativeAppBridge`) so runtime bridge/controller glue no longer imports
  `radiant::app` types directly.
- Native installer/updater shells and `app_core::native_shell` now consume
  migration-facing `app_core::actions` model aliases (browser/map/folder/update/
  waveform + status/prompt types), removing direct `radiant::app` imports from
  migration host entrypoints and projection glue.
- Native runtime-facing projection/view constants now consume migration-facing
  `app_core::{view_model,ui}` aliases instead of direct `app` module paths.
- `app_core::view_model` now exposes a narrowed migration-facing helper surface
  (currently `sample_display_label`) instead of a blanket re-export of legacy
  `app::view_model` symbols.
- `app_core::ui` now owns native viewport baseline constants directly
  (960x560 default, 640x400 minimum) so runtime entrypoints no longer depend on
  legacy `app::ui` constant definitions.
- `app_core::ui` now also owns native projection render caps
  (`MAX_RENDERED_BROWSER_ROWS`, `MAX_RENDERED_MAP_POINTS`) so native-shell
  projection limits are shared from backend-neutral migration constants.
- Native bridge status flows now route directly through
  `app::controller::AppController` methods in `app_core` runtime glue,
  without a separate `set_error_status` migration shim.
- Sempal runtime options now use a sempal-owned `NativeRunOptions` surface in
  `src/gui_runtime/mod.rs` with conversion wrappers into radiant internals,
  removing legacy `EguiRunOptions` naming from sempal's public runtime API.
- Sempal runtime icon payloads now use a sempal-owned `WindowIconRgba` type in
  `src/gui_runtime/mod.rs`, with explicit conversion into radiant runtime
  internals to keep native host API ownership in sempal.
- Sempal `gui_runtime` now exposes a native-only host surface
  (`NativeRunOptions`, `run_native_vello_app`, `run_native_vello_preview`)
  and no longer re-exports egui runtime launch APIs.
- Waveform rendering internals now use backend-neutral image/color buffers
  (`WaveformImage`, `WaveformRgba`) with egui conversion isolated to legacy
  egui-controller call sites.

## Classic Baseline Layout Contract (v2)

The native shell is now tuned against the classic Sempal density baseline:

- standard tier sidebar width is constrained to a compact classic range
  (`sidebar_ratio` 0.14..0.18, `sidebar_max_width` <= 220),
- browser rows target dense table cadence
  (`browser_row_height` 15.5..17.0 with tighter row gaps),
- waveform/header split stays compact to preserve browser table capacity
  (`waveform_ratio` <= 0.36),
- typography remains compact for list readability
  (`font_body` <= 9.1, `font_meta` <= 8.8),
- per-row bucket chip labels are sourced from projection metadata (e.g. BPM badges),
- reference viewport contract uses 1440x810 with:
  sidebar width 155..220, waveform card height 150..280, and >=22 visible browser rows.

This contract is enforced by native-shell style tests in `vendor/radiant/src/gui/native_shell/style.rs`
plus layout geometry tests in `vendor/radiant/src/gui/native_shell/mod.rs`,
and row-label rendering tests in `vendor/radiant/src/gui/native_shell/state.rs`.

## Source Management Polish Checklist

- [x] Keep folder rename prompt open on validation/runtime error instead of collapsing immediately.
- [x] Surface folder create/rename validation errors inside native prompt (`input_error` projection).
- [x] Gate prompt confirm actions when prompt validation errors are present (mouse + keyboard paths).
- [x] Disable recovery-log clear action while recovery is still running.
- [x] Add native-shell tests for disabled source actions and validation-gated prompt confirms.
- [x] Add projection tests for folder create/rename validation errors and recovery-action gating.
- [x] Finalize remaining visual polish for folder recovery affordances across compact viewport edge cases.
