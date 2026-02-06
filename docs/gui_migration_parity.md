# GUI migration parity matrix (`egui` -> `radiant`)

This document tracks feature parity work required to remove Sempal's legacy
`egui` renderer path and run fully on `radiant` (`native_vello`) by default.

## P0 (required before removing legacy runtime path)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Runtime startup | Native backend as default path | Done | Sempal |
| Sources panel | Select source row + reflect missing state | Done (bridge projection) | Sempal + Radiant |
| Browser panel | Row focus and multi-select (click/ctrl/shift patterns) | Done (controller wired) | Sempal |
| Browser panel | Search query update and busy indicator | Done (bridge projection + action) | Sempal |
| Waveform | Seek/cursor/selection/zoom + loop toggle | Done (bridge action mapping) | Sempal |
| Transport | Play/pause toggle | Done | Sempal |
| Undo stack | Undo/redo shortcuts from native runtime | Done | Sempal |
| Status surface | Status text visible in native shell | Done (single-line footer text) | Radiant |
| Focus model | Focus browser/sources/waveform/search targets | Done | Sempal |

## P1 (close parity after legacy path removal)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Browser actions | Context menus (rename/tag/delete) | Not yet in native shell | Radiant + Sempal |
| Source management | Folder actions (rename/create/delete/recovery) | Not yet in native shell | Radiant + Sempal |
| Workflow overlays | Progress, drag overlays, prompts | Partial/legacy-only | Radiant |
| Update UX | In-app release notes/update prompts | Legacy-only | Radiant + Sempal |
| Map view | Cluster map interactions and rendering | Legacy-only | Radiant + Sempal |

## P2 (post-cutover polish and expansion)

| Area | Capability | Current state | Owner target |
| --- | --- | --- | --- |
| Rendering polish | Motion/styling refinement inspired by Xilem/Vello | In progress | Radiant |
| Scale behavior | Browser virtualization/perf tuning beyond 48 rendered rows | Baseline only | Radiant |
| Tooling | Snapshot + interaction golden tests for native shell | Partial | Radiant + Sempal |

## Migration notes

- New backend-neutral projection helpers now live under `src/app_core`.
- Native bridge orchestration remains in `src/gui_app/bridge.rs`.
- Legacy `egui` renderer code remains for now and will be removed after P0 signoff.
