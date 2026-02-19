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

## Ownership and CODEOWNERS

`docs/ARCHITECTURE.md` is the human/agent routing map (source of truth for "where should this change go?").
`.github/CODEOWNERS` is the enforcement mechanism that makes those ownership buckets show up in PR review.

When changing ownership boundaries:

1. Update `docs/ARCHITECTURE.md` (the map) to reflect the intended responsibility split.
2. Update `.github/CODEOWNERS` (the enforcement) to match the same buckets.
3. Prefer broad, stable directory patterns over fragile file-level ownership unless you have a clear need.
4. Keep changes small and reviewable: ownership churn causes review noise and slows throughput.

## Guardrails and invariants

- `src` owns domain state and UI intent only; `vendor/radiant` owns GUI behavior.
- `scripts/check_migration_boundary.sh` enforces the `app_core` migration boundary:
  - Direct `crate::app::` references are only allowed in `src/app_core/app_api.rs`.

## Where tests should go

- Unit tests: next to the logic in `src/**` using `#[cfg(test)]`.
- Integration tests: `tests/`.
- Radiant UI behavior tests and fixtures: `vendor/radiant` (see `docs/TEST.md` for commands).

## Codebase map (short)

- `src/` modules:
  - `analysis` — DSP/feature analysis and similarity tooling.
  - `app` — legacy app model state and controller logic.
  - `app_core` — backend-neutral app-core projection/action helpers.
  - `app_dirs` — filesystem paths for app data, caches, and settings.
  - `audio` — playback, I/O, recording, decoding, and processing.
  - `external_clipboard.rs` — platform clipboard integrations.
  - `external_drag.rs` — platform drag-and-drop integrations.
  - `gui` — new/active GUI abstraction layer for widgets and views.
  - `gui_app` — app-level GUI wiring used by the native runtime.
  - `gui_runtime` — runtime bridge glue for native rendering/input.
  - `http_client.rs` — HTTP client helpers and request utilities.
  - `issue_gateway` — issue reporting and GitHub issue flow.
  - `legacy_runtime` — compatibility/runtime helpers for older shell paths.
  - `logging.rs` — logging setup and tracing helpers.
  - `sample_sources` — sample catalog, scan, and database integration.
  - `selection.rs` — selection helpers and focus state math.
  - `sqlite_ext.rs` — custom SQLite extension loading and helpers.
  - `updater` — update checking, download, install, and patch flow.
  - `wav_sanitize.rs` — WAV header/corpus sanitization helpers.
  - `waveform` — waveform decoding, rendering, and caching.
  - `main.rs` and `bin/` entrypoints — app and updater binaries.
- `vendor/radiant/`: UI shell, layout, and rendering engine used by native shells.
  - Layout contract: `docs/radiant_slot_layout_spec.md`.
  - `vendor/radiant/src/app/` — native-app bridge model types and action enums.
  - `vendor/radiant/src/gui/` — retained shell layout, style, painting, and input bridge.
    - `gui/native_shell/` — interaction state, layout primitives, and shell frame generation.
    - `gui/input.rs` — key/mouse input tokenization consumed by state + actions.
    - `gui/types.rs` — shared geometry and color primitives.
    - `gui/repaint.rs` — repaint/dirty-bit signal bridging into retained caches.
  - `vendor/radiant/src/gui_runtime/` — runtime host entrypoint and native window loop integration.
    - `gui_runtime/native_vello.rs` — Vello-based render loop and scene rebuild scheduler.
- `tests/`: integration tests and behavior checks.
- `assets/`: static runtime assets.
- `scripts/`: build/dev helper scripts.
- `docs/`: developer documentation.
- `manual/`: user-facing documentation (usage guide + published site).

### Submodules

- `vendor/radiant` (UI framework dependency).
