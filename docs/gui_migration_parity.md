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
| Update UX | In-app release notes/update prompts | Legacy-only ancillary UI | Radiant + Sempal |
| Map view | Cluster map interactions and rendering | Legacy-only modules | Radiant + Sempal |

## P2 (post-cutover polish and expansion)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Rendering polish | Motion/styling refinement inspired by Xilem/Vello | In progress (classic-shell browser chrome now uses explicit tab/toolbar/search/chip/header compositions plus two-row top-bar controls + table striping refinements) | Radiant |
| Layout contract | Tokenized header/body/footer geometry shared by paint + hit testing | In progress (browser tabs/toolbar/header/footer heights + table columns now token-driven) | Radiant |
| Sidebar layout | Tokenized source/folder section sizing and action controls | Done (tiered sizing and compact edge-case guards) | Radiant |
| Scale behavior | Browser virtualization/perf tuning beyond 48 rendered rows | In progress (single-table focused window + higher per-tier row caps) | Radiant |
| Tooling | Snapshot + interaction golden tests for native shell | In progress (deterministic frame-contract + virtualization hit/geometry tests) | Radiant + Sempal |

## Migration notes

- New backend-neutral projection helpers now live under `src/app_core`.
- Native bridge orchestration remains in `src/gui_app/bridge.rs`.
- Main runtime backend selection has been removed; `src/main.rs` now boots native Vello directly.
- Native shell layout now derives panel/frame metrics from shared style tokens (`vendor/radiant/src/gui/native_shell/style.rs`) and exposes explicit panel bands in `vendor/radiant/src/gui/native_shell/layout.rs`.
- Browser region migration has started from triage columns toward the classic table shell:
  tabs + toolbar + table header/rows/footer are now explicit layout bands used by paint and hit-testing.
- Browser chrome now renders explicit search + state/sort chips and tokenized table columns (`#`, `Sample`, `Bucket`) instead of a single placeholder toolbar/header text line.
- Top bar now uses a tokenized split layout (title row + controls row) with explicit options/volume meter geometry instead of hardcoded text offsets.
- Installer/updater binaries still use the `egui` host path and are tracked separately.

## Source Management Polish Checklist

- [x] Keep folder rename prompt open on validation/runtime error instead of collapsing immediately.
- [x] Surface folder create/rename validation errors inside native prompt (`input_error` projection).
- [x] Gate prompt confirm actions when prompt validation errors are present (mouse + keyboard paths).
- [x] Disable recovery-log clear action while recovery is still running.
- [x] Add native-shell tests for disabled source actions and validation-gated prompt confirms.
- [x] Add projection tests for folder create/rename validation errors and recovery-action gating.
- [x] Finalize remaining visual polish for folder recovery affordances across compact viewport edge cases.
