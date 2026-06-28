
⚠️ **Warning:** Early alpha software. Use at your own risk, this tool can modify, rename, or delete files, and bugs could damage your sample library. Keep backups and proceed with caution. ⚠️

# WAVECRATE

Audio sample triage tool built with Rust.

[![Wavecrate nightly release](https://github.com/PORTALSURFER/wavecrate/actions/workflows/release-build.yml/badge.svg)](https://github.com/PORTALSURFER/wavecrate/actions/workflows/release-build.yml)

---

[![Sample workflow preview](assets/example.gif)](assets/example.gif)

[![Buy Me A Coffee](https://img.buymeacoffee.com/button-api/?text=Buy%20me%20a%20coffee&slug=portalsurfer&button_colour=FFDD00&font_colour=000000&font_family=Inter&outline_colour=000000&coffee_colour=ffffff)](https://buymeacoffee.com/portalsurfer)

## Downloads

- The GitHub Actions release lane publishes rolling nightly builds only.
- The nightly workflow builds Windows x86_64 plus macOS x86_64 and aarch64
  assets from `main`.
- It runs every evening and can also be started manually from the GitHub Actions
  page to force a fresh nightly from the current `main` commit.
- The workflow uploads the built zips and generated Markdown release log through
  the scoped PortalSurfer upload API so the `/wavecrate/` frontend can show the
  latest downloadable build and changelog.

## Build from source

- Requires Rust (stable toolchain) and `cargo`.
- Initialize submodules after clone: `git submodule update --init --recursive`.
- For the normal local app loop from the project root: `cargo run`.
- The repo-root `./run.sh` and `.\run.ps1` launch the same Cargo path for
  compatibility with older local muscle memory.
- For exact shipping-profile behavior: `cargo run -p wavecrate --bin wavecrate --release`.
- Or build once and run the shipping binary: `cargo build -p wavecrate --release` then `target/release/wavecrate`.
- Playback uses your default audio output device.
- GUI backend:
  - Main app uses the Radiant application path in `src/native_app.rs`.
  - Current product folder/file drag/drop behavior lives in
    `src/native_app/sample_library/folder_browser/**` and
    `src/native_app/sample_library/drag_drop_actions/**`.
  - `src/app/controller/ui/drag_drop_controller/**` is compatibility-controller
    drag/drop code, not the default place for current GUI drag/drop fixes.
  - `app_core` defines the domain action/projection layer used by tests and companion runtime surfaces without owning widget behavior.
  - Architecture and target guidance: `docs/TARGET.md`.
- Windows (ASIO): If you want to build with ASIO support (or your build fails looking for the ASIO SDK), download the Steinberg ASIO SDK and set `CPAL_ASIO_DIR` to the SDK path (e.g. a folder named `ASIOSDK`) before running `cargo build`/`cargo run`.

Local CI parity command (canonical):
- macOS/Linux/WSL: `bash scripts/ci.sh local`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 local`

Fast local app loop:
- macOS/Linux/WSL: `bash scripts/ci.sh smoke`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 smoke`
- local app runs: `cargo run`

Agent-safe local validation loop for constrained environments:
- macOS/Linux/WSL: `bash scripts/ci.sh agent`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
- Runs `devcheck` plus `cargo test -p wavecrate --lib -- --test-threads=1` without `cargo nextest`.

Broader integrated local validation loop:
- macOS/Linux/WSL: `bash scripts/ci.sh quick`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
- Built around `cargo nextest`; the Windows PowerShell wrapper also runs the semantic GUI contract lane.

The CI-parity scripts run formatting, linting, docs, tests, and perf guardrails
in the canonical local validation order.
On headless Linux hosts, `scripts/ci.sh local` and `scripts/perf.sh guard`
automatically set `ALSA_CONFIG_PATH=scripts/internal/alsa_headless.conf` (unless already
set) to route audio probing to a dummy sink and reduce ALSA warning noise.

## Architecture

- `vendor/radiant` owns GUI abstractions, widgets, layout, input handling, diff/update
  logic, and rendering coordination.
- `src/gui` re-exports backend-neutral `radiant` GUI primitives for app code.
- `src/gui_runtime` is the thin host-bridge layer between Wavecrate and `radiant`.
- `src/app_core` is the stable application-core projection and state transition surface used by the native runtime and automation/test hosts.
- `src/app` still owns the compatibility-heavy application model/controller logic that has not yet migrated into `app_core`.
- Core domain logic, sample indexing, playback, and persistence remain in `src/` modules
  (`audio`, `analysis`, `sample_sources`, `updater`, etc.).
- Rendering is performed by the `vello` backend through the `radiant` runtime path.

In practice, `src` should declare UI intent through `radiant` and avoid duplicating
GUI helpers, layout logic, hit-testing, and event routing logic that belongs in
`radiant`.

Radiant update/diff model:

- App code builds declarative state for UI intents (`actions`, `projection`, and view
  shape) and passes it through the host bridge.
- `radiant` computes the structural diff between the current and next UI trees.
- Only changed subtrees are invalidated and repainted.
- Expensive rasterization and paint graph construction stay inside the `radiant`
  runtime, while `src` only controls when new state should be scheduled.

Event propagation model:

- Input is normalized in `radiant` from native events into key/mouse tokens.
- The framework performs deterministic hit-testing from layout bounds before routing.
- Focus, pointer capture, keyboard shortcuts, and drag sequences are resolved by
  `radiant`.
- App-level callbacks receive action payloads and apply domain-level state updates in
  response; they do not mutate widget internals.

Contract notes for contributors:

- `src` owns domain state and intent only:
  - Construct state transitions and action payloads.
  - Trigger domain operations (analysis, playback, persistence, etc.).
  - Render intent is expressed via `radiant` types and projections.
- `radiant` owns all GUI behavior:
  - Widget behavior, layout policies, focus model, hit testing, and event propagation.
  - Diff/update reconciliation and scene invalidation decisions.
  - Scene/paint scheduling and Vello render orchestration.
- The runtime adapter (`src/gui_runtime`) and native entrypoint (`src/main.rs`) are thin adapters:
  - Convert app runtime options into `radiant` runtime options.
  - Forward errors for diagnostics.
  - Do not embed widget semantics or layout policy.

For deterministic input behavior, keep event handling inside `radiant` entry points and
avoid duplicate state mutation in UI callbacks.

## Optional performance instrumentation

- Rendering instrumentation is opt-in via cargo feature:
  - `gui-performance` (build with `--features gui-performance`)
  - `native-bridge-metrics` (build with `--features native-bridge-metrics`) for host bridge timing
- Enable runtime logs by setting:
  - `RADIANT_NATIVE_RENDER_PROFILE=true`
  - `WAVECRATE_NATIVE_BRIDGE_PROFILE=true`
  - Accepts `1`, `true`, `on`, and `yes` (case-insensitive for `TRUE`/`On`/`ON` style variants)
- Logging includes frame build, model access, motion overlay, and frame-submit timing for render; bridge profiling also includes adapter transition and event-funnel timing.
  plus per-frame scene/motion/static rebuild and text cache counters.
- Default builds keep all profiling call sites in the no-op fast path.

## ML model setup (PANNs)

PANNs embedding inference is deprecated and removed in this codebase (see
`src/analysis/mod.rs`). Current builds do not require downloading/exporting a
PANNs ONNX model.

Legacy golden regression tests still use PANNs reference artifacts:

- Generate/update golden references:
  - `bash scripts/check.sh golden-tests`
  - `powershell -ExecutionPolicy Bypass -File scripts/check.ps1 golden-tests`
- These scripts set:
  - `WAVECRATE_PANNS_GOLDEN_PATH`
  - `WAVECRATE_PANNS_EMBED_GOLDEN_PATH`

## Code style and linting

- Install tooling once: `rustup component add rustfmt clippy`.
- Format locally: `cargo fmt --all`.
- Check formatting: `cargo fmt --all -- --check`.
- Lint locally: `cargo clippy --workspace --all-targets`.
- Run the local validation wrappers before release-risk changes; GitHub Actions
  currently owns nightly packaging and upload, not push/PR validation.

## Configuration and data

- Each source folder gets a hidden `.wavecrate_samples.db` that tracks indexed `.wav` files and their tags.
- Wavecrate alpha app builds currently support macOS and Windows. Linux and WSL references in this repository are for development and CI only, not supported app installs.
- App files live in a single `.wavecrate` folder inside the supported OS config directory:
  - Windows: `%APPDATA%\\.wavecrate\\config.toml`
  - macOS: `~/Library/Application Support/.wavecrate/config.toml`
- Non-live persistence profiles live under the same root at `.wavecrate/profiles/<name>`.
- `sandbox` is the canonical manual-QA profile; `automated-tests` is the canonical automated-validation profile.
- Automated test and GUI-validation flows default to isolated non-live profiles so they do not rewrite the live `library.db` source list.
- Manual GUI validation against the real persisted startup path is still available by opting into the `live` GUI fixture explicitly.

## SQLite extensions (optional)

- Wavecrate can load a SQLite extension for faster vector operations via `WAVECRATE_SQLITE_EXT`.
- Loading is opt-in with `WAVECRATE_SQLITE_EXT_ENABLE=1` and restricted to `<app_root>/sqlite_extensions`.
- Unsafe mode (`WAVECRATE_SQLITE_EXT_UNSAFE=1`) bypasses the allowlist, but it is ignored unless the build enables the `sqlite-ext-unsafe` cargo feature.
- If you need unsafe mode, rebuild with `cargo build --release --features sqlite-ext-unsafe` and supply a fully trusted extension path.

## Logging

- Startup initializes console logging and a per-launch log file under the same `.wavecrate` folder:
  - Windows: `%APPDATA%\\.wavecrate\\logs`
  - macOS: `~/Library/Application Support/.wavecrate/logs`
- Log filenames include the launch timestamp, and the 10 most recent files are retained by pruning the oldest.

## Documentation

- [Usage guide](manual/usage.md)
- Developer docs entry point: `docs/README.md`
