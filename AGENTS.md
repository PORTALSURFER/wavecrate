
when building you will need asiosdk, you can find it at /mnt/e/lib/asiosdk/ASIOSDK, map this to CPAL_ASIO_DIR env var
refer to /docs/design_principles.md when implementing change requests

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
- `tests/`: integration tests and behavior checks.
- `assets/`: static runtime assets.
- `scripts/`: build/dev helper scripts.
- `docs/`: design/docs references and implementation notes.

### Submodules

- `vendor/radiant` (UI framework dependency).
