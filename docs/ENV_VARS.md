# Environment Variables

This document lists runtime and build-related environment variables used by
Sempal. Unless noted otherwise, variables are optional.

## Conventions

- Booleans: prefer setting `1` for “enabled” unless the variable explicitly
  documents additional accepted values.
- Paths: prefer absolute paths. Relative paths are interpreted by the code that
  consumes them (often relative to the current working directory).

## Build and release

- `CPAL_ASIO_DIR`
Path to the Steinberg ASIO SDK root directory (expected to contain `host/` and
`common/`). Required only when building with ASIO support on Windows.

- `SEMPAL_CARGO_BIN`
Overrides the build tool used by `scripts/build_release_zip.sh` (defaults to
`cargo`). CI sets this to `cross` for cross builds.

- `SEMPAL_CHECKSUMS_ED25519_KEY`
CI secret used by `.github/workflows/release-build.yml` to sign release checksum
files. This is expected to be an Ed25519 private key in PEM form that OpenSSL
can use for `pkeyutl -sign`.

## Paths and directories

- `SEMPAL_CONFIG_HOME`
Overrides the OS config base directory used for app files. Sempal still creates
and uses a `.sempal` directory under this base.

Example: `SEMPAL_CONFIG_HOME=/tmp` causes logs to be written under
`/tmp/.sempal/logs` on Unix-like platforms.

- `SEMPAL_NATIVE_FONT_PATH`
Optional path to a `.ttf` font used by the native shell text renderer when
system font discovery fails.

## Logging

- `RUST_LOG`
Controls logging verbosity via `tracing_subscriber::EnvFilter`. When unset,
Sempal defaults to `info`.

## UI and runtime profiling

- `SEMPAL_NATIVE_RENDER_PROFILE`
Enables native render profiling logs when the build enables the `gui-performance`
cargo feature. Accepted values include `1`, `true`, `on`, and `yes` (case
variants are accepted in the current implementation).

- `SEMPAL_NATIVE_BRIDGE_PROFILE`
Enables native bridge profiling logs when the build enables the
`native-bridge-metrics` cargo feature. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive).

## Analysis job tuning and debugging

- `SEMPAL_ANALYSIS_WORKERS`
Overrides the analysis worker count (integer >= 1).

- `SEMPAL_DECODE_WORKERS`
Overrides the decode worker count (integer >= 1).

- `SEMPAL_ANALYSIS_CLAIM_BATCH`
Overrides the analysis job claim batch size (integer >= 1). Default: `64`.

- `SEMPAL_DECODE_QUEUE_TARGET`
Overrides the decode queue target size (integer >= 1).

- `SEMPAL_ANALYSIS_STALE_SECS`
Overrides the “stale running job” timeout in seconds (integer >= 60). Default:
`120`.

- `SEMPAL_ANALYSIS_LOG_JOBS`
Enables additional analysis worker logging when set to `1`.

- `SEMPAL_ANALYSIS_LOG_QUEUE`
Enables additional analysis queue logging when set to `1`.

## SQLite extension loading (optional)

- `SEMPAL_SQLITE_EXT`
Path to a loadable SQLite extension (`.so`, `.dll`, `.dylib`).

- `SEMPAL_SQLITE_EXT_ENABLE`
Must be set to a truthy value (for example `1`) to opt in to loading
`SEMPAL_SQLITE_EXT`.

- `SEMPAL_SQLITE_EXT_UNSAFE`
Requests unsafe loading mode (bypasses allowlist and relaxes safety checks).
This is ignored unless the build enables the `sqlite-ext-unsafe` cargo feature.

## Updater safety overrides

- `SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS`
Dev/test override that allows the updater to proceed when symlink validation
fails due to I/O or permission errors. Accepted values: `1`, `true`, `yes`
(case-insensitive).

## Tests and ad-hoc debugging

- `SEMPAL_TEST_AUDIO_OUTPUT`
Windows test override. When running tests on Windows, set `SEMPAL_TEST_AUDIO_OUTPUT=1`
to exercise the real audio output device; otherwise tests avoid opening audio
devices to reduce driver crash risk.

- `SEMPAL_TRANSIENT_DEBUG`
When set (to any value), enables transient detection debug logs.

- `SEMPAL_PANNS_GOLDEN_PATH`
Path to the golden log-mel JSON used by the golden regression tests (typically
set by `scripts/ci_golden_tests.sh` / `scripts/ci_golden_tests.ps1`).

- `SEMPAL_PANNS_EMBED_GOLDEN_PATH`
Path to the golden embedding JSON used by the golden regression tests (typically
set by `scripts/ci_golden_tests.sh` / `scripts/ci_golden_tests.ps1`).

## Issue reporting token storage

These variables affect the GitHub issue token storage backend in
`src/issue_gateway/token_store.rs`.

- `SEMPAL_DISABLE_KEYRING`
Disables OS keyring usage when set to a truthy value.

- `SEMPAL_ALLOW_FALLBACK_TOKEN_STORAGE`
Opt-in switch that permits encrypted on-disk fallback token storage when keyring
operations fail or are unavailable.

- `SEMPAL_FALLBACK_KEY`
Supplies the 32-byte fallback encryption key via an environment variable (useful
when keyring storage is unavailable). This is expected to be base64-encoded.

## PANNs model setup (README-only variables)

The README references these variables for PANNs model setup and controlled ONNX
downloads. At the time of writing, only the README references them in-tree.

- `SEMPAL_PANNS_ONNX_PATH`
- `SEMPAL_MODELS_DIR`
- `SEMPAL_PANNS_ONNX_URL`
- `SEMPAL_PANNS_ONNX_SHA256`
- `SEMPAL_PANNS_ONNX_ALLOWED_HOSTS`

