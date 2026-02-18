# Architecture and Module Ownership

This document is a lightweight “where should this change go?” map for Sempal.
It is optimized for quick routing decisions by humans and coding agents.

If you are implementing a change request, read `docs/design_principles.md` and
use `docs/FEATURE_CHECKLIST.md` as the default safe path.

## Change routing rules

- Domain logic (indexing, analysis, playback behavior, persistence): put it in `src/` modules.
- UI behavior (widgets, layout policies, focus model, hit testing, input routing): put it in `vendor/radiant/`.
- App UI wiring (turn domain state into UI intent and actions): put it in `src/gui`, `src/gui_app`, `src/gui_runtime`, and `src/app_core`.
- Avoid adding new behavior to legacy UI/controller paths in `src/app` unless you are intentionally working in the legacy runtime.

## Ownership map (by responsibility)

- App core projection and backend-neutral intent: `src/app_core`
- Legacy app model/controller: `src/app`
- GUI abstraction and widgets (app-side): `src/gui`
- App-level GUI wiring (native runtime): `src/gui_app`
- Runtime host bridge glue: `src/gui_runtime`
- UI framework, retained layout, input normalization, rendering coordination: `vendor/radiant`
- Audio playback, I/O, decoding, processing: `src/audio`
- DSP/feature analysis, similarity tooling, ANN containers: `src/analysis`
- Sample catalog, scanning, database integration: `src/sample_sources`
- Selection math and focus helpers: `src/selection.rs`
- Filesystem paths for app data/caches/settings: `src/app_dirs`
- Update checking/download/install flow: `src/updater` and `src/bin/sempal-updater`
- Installer UI: `src/bin/sempal-installer`
- Logging setup and tracing helpers: `src/logging.rs`
- HTTP request helpers: `src/http_client.rs`
- Issue reporting and GitHub issue flow: `src/issue_gateway`
- Optional SQLite extension loading: `src/sqlite_ext.rs`
- Platform clipboard/drag-and-drop integrations: `src/external_clipboard.rs`, `src/external_drag.rs`

## Guardrails and invariants

- `src` owns domain state and UI intent only; `vendor/radiant` owns GUI behavior.
- `scripts/check_migration_boundary.sh` enforces the `app_core` migration boundary:
  - Direct `crate::app::` references are only allowed in `src/app_core/app_api.rs`.

## Where tests should go

- Unit tests: next to the logic in `src/**` using `#[cfg(test)]`.
- Integration tests: `tests/`.
- Radiant UI behavior tests and fixtures: `vendor/radiant` (see `docs/TEST.md` for commands).
