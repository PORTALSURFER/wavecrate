
when building you will need asiosdk, you can find it at /mnt/e/lib/asiosdk/ASIOSDK, map this to CPAL_ASIO_DIR env var
refer to `docs/design_principles.md` when implementing change requests
Windows logs can be found at `/mnt/c/Users/wanja.svasek/AppData/Roaming/.sempal/logs`.

## How to add a feature safely

See `docs/FEATURE_CHECKLIST.md`.

## Environment variables

See `docs/ENV_VARS.md`.

## Version control rule

After any code change, create a commit and push it.
If your environment requires explicit approval for git operations, ask for confirmation and include the intended commit message.

## Architecture and module ownership

See `docs/ARCHITECTURE.md`.

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
  - `README.md` — developer documentation entry point.
  - `FEATURE_CHECKLIST.md` — safe path for implementing changes.
  - `ARCHITECTURE.md` — module ownership map.
  - `ENV_VARS.md` — environment variable reference.
  - `TEST.md` — test suite inventory and command guide.
- `manual/` docs map (user-facing documentation only).
  - `index.md` — project homepage content, overview, and documentation entry point.
  - `usage.md` — user-facing usage guide and feature walkthrough.
  - `_config.yml` — GitHub Pages/Jekyll documentation site configuration.
  - `_layouts/default.html` — docs site page shell and common header/footer layout.
  - `assets/theme.css` — documentation theme and layout styling.
  - `assets/screenshot.png` — brand/appearance reference image for docs.

### Submodules

- `vendor/radiant` (UI framework dependency).
