
⚠️ **Warning:** Early alpha software. Use at your own risk, this tool can modify, rename, or delete files, and bugs could damage your sample library. Keep backups and proceed with caution. ⚠️

# SEMPAL

Audio sample triage tool built with Rust.

[![Build release assets](https://github.com/PORTALSURFER/sempal/actions/workflows/release-build.yml/badge.svg)](https://github.com/PORTALSURFER/sempal/actions/workflows/release-build.yml)

---

[![Sample workflow preview](assets/example.gif)](assets/example.gif)

[![Buy Me A Coffee](https://img.buymeacoffee.com/button-api/?text=Buy%20me%20a%20coffee&slug=portalsurfer&button_colour=FFDD00&font_colour=000000&font_family=Inter&outline_colour=000000&coffee_colour=ffffff)](https://buymeacoffee.com/portalsurfer)

## Downloads

- Windows binaries are published on GitHub Releases (Windows only for now).
- Publishing a release triggers a workflow that regenerates `CHANGELOG.md` via `git-cliff` and opens a PR (since `main` is protected).

## Build from source

- Requires Rust (stable toolchain) and `cargo`.
- Initialize submodules after clone: `git submodule update --init --recursive`.
- For the fastest local app loop from the project root: `cargo run-fast`.
- For exact shipping-profile behavior: `cargo run -p sempal --bin sempal --release`.
- Or build once and run the shipping binary: `cargo build -p sempal --release` then `target/release/sempal`.
- Playback uses your default audio output device.
- GUI backend:
  - Main app uses `native_vello` (radiant runtime path) by default and no longer exposes a legacy backend switch.
  - `app_core` defines the domain action/projection layer that feeds `radiant` without owning widget behavior.
  - Migration parity tracker: `docs/gui_migration_parity.md`.
  - Native shell text rendering can use `SEMPAL_NATIVE_FONT_PATH=/path/to/font.ttf` if automatic system font discovery fails.
- Windows (ASIO): If you want to build with ASIO support (or your build fails looking for the ASIO SDK), download the Steinberg ASIO SDK and set `CPAL_ASIO_DIR` to the SDK path (e.g. a folder named `ASIOSDK`) before running `cargo build`/`cargo run`.

Local CI parity command (canonical):
- macOS/Linux/WSL: `bash scripts/ci_local.sh`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`

Fast local app loop:
- macOS/Linux/WSL: `bash scripts/devcheck.sh`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- local release-like runs: `cargo run-fast`

Agent-safe local validation loop for constrained environments:
- macOS/Linux/WSL: `bash scripts/ci_agent.sh`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
- Runs `devcheck` plus `cargo test -p sempal --lib -- --test-threads=1` without `cargo nextest`.

Broader integrated local validation loop:
- macOS/Linux/WSL: `bash scripts/ci_quick.sh`
- Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Built around `cargo nextest`; the Windows PowerShell wrapper also runs the semantic GUI contract lane.

The CI-parity scripts run formatting, linting, docs, tests, and perf guardrails
in the same order used by repository workflows.
On headless Linux hosts, `scripts/ci_local.sh` and `scripts/run_perf_guard.sh`
automatically set `ALSA_CONFIG_PATH=scripts/alsa_headless.conf` (unless already
set) to route audio probing to a dummy sink and reduce ALSA warning noise.

## Architecture

- `vendor/radiant` owns GUI abstractions, widgets, layout, input handling, diff/update
  logic, and rendering coordination.
- `src/gui` contains the app-level GUI wiring for Sempal and consumes the `radiant` API.
- `src/gui_runtime` is the host-bridge layer between Sempal and `radiant`.
- `src/app_core` is the stable application-core projection and state transition surface used by both GUI and other runtimes.
- `src/legacy_runtime` remains for compatibility paths; new runtime behavior should be additive and routed through `radiant`.
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
- Host bridges (`src/gui_app`, `src/gui_runtime`) are thin adapters:
  - Convert app runtime options into `radiant` run-time options.
  - Forward errors for diagnostics.
  - Do not embed widget semantics or layout policy.

For deterministic input behavior, keep event handling inside `radiant` entry points and
avoid duplicate state mutation in UI callbacks.

## Optional performance instrumentation

- Rendering instrumentation is opt-in via cargo feature:
  - `gui-performance` (build with `--features gui-performance`)
  - `native-bridge-metrics` (build with `--features native-bridge-metrics`) for host bridge timing
- Enable runtime logs by setting:
  - `SEMPAL_NATIVE_RENDER_PROFILE=true`
  - `SEMPAL_NATIVE_BRIDGE_PROFILE=true`
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
  - `bash scripts/ci_golden_tests.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_golden_tests.ps1`
- These scripts set:
  - `SEMPAL_PANNS_GOLDEN_PATH`
  - `SEMPAL_PANNS_EMBED_GOLDEN_PATH`

## Code style and linting

- Install tooling once: `rustup component add rustfmt clippy`.
- Format locally: `cargo fmt --all`.
- Check formatting (same as CI): `cargo fmt --all -- --check`.
- Lint locally (same as CI): `cargo clippy --workspace --all-targets`.
- CI runs `rustfmt`, `clippy`, and `cargo test` on Ubuntu/Windows/macOS for every push to `main`/`next` and all pull requests targeting those branches.

## Configuration and data

- Each source folder gets a hidden `.sempal_samples.db` that tracks indexed `.wav` files and their tags.
- App files live in a single `.sempal` folder inside your OS config directory:
  - Linux: `$HOME/.config/.sempal/config.toml`
  - Windows: `%APPDATA%\\.sempal\\config.toml`
  - macOS: `~/Library/Application Support/.sempal/config.toml`

## SQLite extensions (optional)

- Sempal can load a SQLite extension for faster vector operations via `SEMPAL_SQLITE_EXT`.
- Loading is opt-in with `SEMPAL_SQLITE_EXT_ENABLE=1` and restricted to `<app_root>/sqlite_extensions`.
- Unsafe mode (`SEMPAL_SQLITE_EXT_UNSAFE=1`) bypasses the allowlist, but it is ignored unless the build enables the `sqlite-ext-unsafe` cargo feature.
- If you need unsafe mode, rebuild with `cargo build --release --features sqlite-ext-unsafe` and supply a fully trusted extension path.

## Logging

- Startup initializes console logging and a per-launch log file under the same `.sempal` folder:
  - Linux: `$HOME/.config/.sempal/logs`
  - Windows: `%APPDATA%\\.sempal\\logs`
  - macOS: `~/Library/Application Support/.sempal/logs`
- Log filenames include the launch timestamp, and the 10 most recent files are retained by pruning the oldest.

## Documentation

- [Usage guide](manual/usage.md)
- Developer docs entry point: `docs/README.md`
