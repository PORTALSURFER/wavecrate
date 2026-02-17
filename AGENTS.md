
when building you will need asiosdk, you can find it at /mnt/e/lib/asiosdk/ASIOSDK, map this to CPAL_ASIO_DIR env var
refer to `design_principles.md` when implementing change requests
Windows logs can be found at `/mnt/c/Users/wanja.svasek/AppData/Roaming/.sempal/logs`.

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
- `docs/`: test and docs support metadata.
  - `TEST.md` — test suite inventory and command guide.
- `manual` docs map (project documentation and design references).
  - `index.md` — project homepage content, overview, and documentation entry point.
  - `usage.md` — user-facing usage guide and feature walkthrough.
  - `design_principles.md` — architectural goals, constraints, and coding standards.
  - `performance_qa.md` — performance targets and QA checks for large datasets/views.
  - `gui_migration_parity.md` — parity matrix for legacy runtime vs `radiant`.
  - `native_shell_legacy_baseline.md` — baseline shell contracts for legacy/native parity.
  - `feature_vector.md` — feature-vector definition and ANN-related metadata.
  - `ann_index_container.md` — ANN index container format and storage design notes.
  - `updater-contract.md` — updater state machine and public contract surface.
  - `hints.md` — hint-of-the-day catalog and messaging patterns.
  - `icon_assets.md` — icon asset generation and asset conventions.
  - `styleguide.md` — GUI style direction and visual language definitions.
  - `plan.md` — ongoing implementation plan for single-file ANN container work.
  - `transient_plan.md` — transient detection improvement plan and milestones.
  - `transient_audit.md` — transient detection audit and current implementation status.
  - `todo.md` — tracked backlog and task list.
  - `_config.yml` — GitHub Pages/Jekyll documentation site configuration.
  - `_layouts/default.html` — docs site page shell and common header/footer layout.
  - `assets/theme.css` — documentation theme and layout styling.
  - `assets/screenshot.png` — brand/appearance reference image for docs.

### Submodules

- `vendor/radiant` (UI framework dependency).
