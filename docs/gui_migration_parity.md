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
| Browser actions | Context menus (rename/tag/delete) | Not yet in native shell | Radiant + Sempal |
| Source management | Folder actions (rename/create/delete/recovery) | Not yet in native shell | Radiant + Sempal |
| Workflow overlays | Progress, drag overlays, prompts | Partial (legacy implementation remains in old modules) | Radiant |
| Update UX | In-app release notes/update prompts | Legacy-only ancillary UI | Radiant + Sempal |
| Map view | Cluster map interactions and rendering | Legacy-only modules | Radiant + Sempal |

## P2 (post-cutover polish and expansion)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Rendering polish | Motion/styling refinement inspired by Xilem/Vello | In progress | Radiant |
| Layout contract | Tokenized header/body/footer geometry shared by paint + hit testing | Done | Radiant |
| Scale behavior | Browser virtualization/perf tuning beyond 48 rendered rows | Baseline only | Radiant |
| Tooling | Snapshot + interaction golden tests for native shell | Partial | Radiant + Sempal |

## Migration notes

- New backend-neutral projection helpers now live under `src/app_core`.
- Native bridge orchestration remains in `src/gui_app/bridge.rs`.
- Main runtime backend selection has been removed; `src/main.rs` now boots native Vello directly.
- Native shell layout now derives panel/frame metrics from shared style tokens (`vendor/radiant/src/gui/native_shell/style.rs`) and exposes explicit panel bands in `vendor/radiant/src/gui/native_shell/layout.rs`.
- Installer/updater binaries still use the `egui` host path and are tracked separately.
