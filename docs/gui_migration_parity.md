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
| Update UX | In-app release notes/update prompts | In progress (native top-bar update actions now include projected status/hint labels plus check/open/install/dismiss controls) | Radiant + Sempal |
| Map view | Cluster map interactions and rendering | In progress (native map tab now renders normalized points with click-to-focus routing plus projected legend/selection/viewport labels) | Radiant + Sempal |

## P2 (post-cutover polish and expansion)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Rendering polish | Motion/styling refinement inspired by Xilem/Vello | In progress (classic-shell browser chrome now uses explicit tab/toolbar/search/chip/header compositions, two-row top-bar controls, stronger alternating table striping, and waveform title hierarchy refinement) | Radiant |
| Layout contract | Tokenized header/body/footer geometry shared by paint + hit testing | In progress (browser tabs/toolbar/header/footer heights + table columns now token-driven, plus tokenized viewport/guard-rail clamps and snapshot contract metrics) | Radiant |
| Sidebar layout | Tokenized source/folder section sizing and action controls | Done (tiered sizing and compact edge-case guards) | Radiant |
| Scale behavior | Browser virtualization/perf tuning beyond 48 rendered rows | In progress (single-table focused window + higher per-tier row caps) | Radiant |
| Tooling | Snapshot + interaction golden tests for native shell | In progress (deterministic frame-contract + virtualization hit/geometry tests + tiered visual-density contract assertions) | Radiant + Sempal |

## Migration notes

- New backend-neutral projection helpers now live under `src/app_core`.
- Native bridge orchestration remains in `src/gui_app/bridge.rs`.
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
- Installer/updater binaries still use the `egui` host path and are tracked separately.

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
