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

- `SEMPAL_WINDOWS_SIGN_PFX_BASE64`
CI secret used by `.github/workflows/release-build.yml` to Authenticode-sign
Windows executables. This value is expected to be a base64-encoded `.pfx`
certificate bundle.

- `SEMPAL_WINDOWS_SIGN_PFX_PASSWORD`
Password for `SEMPAL_WINDOWS_SIGN_PFX_BASE64`, consumed by the Windows signing
step in `.github/workflows/release-build.yml`.

## Agent hand-off guardrails

- `AGENT_PREFLIGHT_UPDATER`
Default updater name used by `run_agent_preflight.sh` when it is refreshing
`MEMORY.md`, and the default required value for the preflight freshness check.
Precedence (highest to lowest):
1) `run_agent_preflight.sh --updater`
2) `AGENT_PREFLIGHT_UPDATER` (default `Codex`).

- `AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS`
Freshness horizon (in hours) used by `run_agent_preflight.sh` unless overridden by
`run_agent_preflight.sh --memory-max-age-hours`.
Precedence (highest to lowest):
1) `run_agent_preflight.sh --memory-max-age-hours`
2) `AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS` (default `1`).

- `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK_INSTALL`
Set to `1` to skip automatic preflight hook installation in bootstrap.
Hook execution for existing hook setups is separately controlled by
`SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1`.

- `MEMORY_MAX_AGE_HOURS`
Override for `scripts/check_memory_log.sh` freshness checks (hours). Defaults to
`24`. `0` is allowed (requires a timestamp no older than the current hour).

- `MEMORY_REQUIRED_UPDATER`
If set, `scripts/check_memory_log.sh` requires `Updated By:` in `MEMORY.md` to
match this exact value.

- `AGENT_CI_MEMORY_MAX_AGE_HOURS`
Default freshness for `run_agent_ci_checks.sh`/`ci_local.sh` when
`--memory-max-age-hours` is not provided. Defaults to `24`.

- `AGENT_CI_REQUIRED_UPDATER`
Optional required updater for `run_agent_ci_checks.sh` and `ci_local.sh` when
`--required-updater` is not provided.

## Paths and directories

- `SEMPAL_CONFIG_HOME`
Overrides the OS config base directory used for app files. Sempal still creates
and uses a `.sempal` directory under this base.

Example: `SEMPAL_CONFIG_HOME=/tmp` causes logs to be written under
`/tmp/.sempal/logs` on Unix-like platforms.

### What writes where (important for safe local runs)

- App config + logs:
  - Sempal writes under `<SEMPAL_CONFIG_HOME>/.sempal/` (or your OS config dir if `SEMPAL_CONFIG_HOME` is unset).
  - `scripts/run_sandbox.*` sets `SEMPAL_CONFIG_HOME` so the run does **not** touch your real user profile config/log directories.
- Per-source-folder database side effect:
  - If you point Sempal at a folder, it may create or update a `.sempal_samples.db` in that source folder.
  - `SEMPAL_CONFIG_HOME` does **not** relocate these per-folder DB files.
  - `scripts/run_sandbox.sh` and `scripts/run_sandbox.ps1` set
    `SEMPAL_SOURCE_DB_READ_ONLY=1` by default, so source DB writes are blocked
    unless `--write-db` is explicitly passed.

### Source database behavior

- `SEMPAL_SOURCE_DB_READ_ONLY`
  - When `1`/`true`/`yes`/`on`, `SourceDatabase::open` uses read-only access by
    default and requires `<source>/.sempal_samples.db` to already exist.
  - When unset/default, normal write-capable behavior applies.

- `SEMPAL_ALLOW_USER_LIBRARY_DB_WRITE`
  - When write mode is enabled, this must be set to `1`/`true`/`yes`/`on` to allow
    DB writes in paths that look like user libraries (for example `~/Music`,
    `~/Documents`, `~/Downloads`, `~/Desktop`).

- `SEMPAL_NATIVE_FONT_PATH`
Optional path to a `.ttf` font used by the native shell text renderer when
system font discovery fails.

## Logging

- `RUST_LOG`
Controls logging verbosity via `tracing_subscriber::EnvFilter`. When unset,
Sempal defaults to `info`.

## UI and runtime profiling

- `SEMPAL_BROWSER_SEARCH_OFFLOAD_THRESHOLD`
Wav-entry threshold used by browser search to switch from synchronous layout
path recompute to async search jobs. Default: `5000`.

- `SEMPAL_NATIVE_RENDER_PROFILE`
Enables native render profiling logs when the build enables the `gui-performance`
cargo feature. Accepted values include `1`, `true`, `on`, and `yes` (case
variants are accepted in the current implementation).

- `SEMPAL_NATIVE_BRIDGE_PROFILE`
Enables native bridge profiling logs when the build enables the
`native-bridge-metrics` cargo feature. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive).

## Performance guard benchmark overrides

- `SEMPAL_PERF_GUARD_OUT`
Output path used by `scripts/run_perf_guard.sh` for the benchmark JSON report.
Default: `target/perf/bench.json`.

- `SEMPAL_PERF_GUARD_GUI_ROWS`
Synthetic GUI row count passed to `sempal-bench` for the guard run.
Default: `2500`.

- `SEMPAL_PERF_GUARD_GUI_INTERACTION_ROWS`
Synthetic row count used for focused interaction benchmark scenarios.
Default: `1500`.

- `SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS`
Measured iteration count per focused interaction scenario.
Default: `24`.

- `SEMPAL_PERF_GUARD_WARMUP_ITERS`
Warmup iteration count used during guard benchmark runs.
Default: `3`.

- `SEMPAL_PERF_GUARD_MEASURE_ITERS`
Measured iteration count used for each guard benchmark action.
Default: `16`.

- `SEMPAL_PERF_GUARD_RUNS`
Number of full `sempal-bench` runs executed by `scripts/run_perf_guard.sh`.
When greater than `1`, the guard reports median percentiles across runs and the
p95 spread across runs. Default: `1`.

- `SEMPAL_PERF_WARN_P95_US_HOVER`
Warning-only p95 latency threshold (microseconds) for hover interaction
benchmark results. Default: `8000`.

- `SEMPAL_PERF_WARN_P95_US_FILTER_CHURN`
Warning-only p95 latency threshold (microseconds) for filter-only browser churn
benchmark results. Default: `10000`.

- `SEMPAL_PERF_WARN_P95_US_QUERY_CHURN`
Warning-only p95 latency threshold (microseconds) for query-only browser churn
benchmark results. Default: `12000`.

- `SEMPAL_PERF_WARN_P95_US_SORT_CHURN`
Warning-only p95 latency threshold (microseconds) for sort-only browser churn
benchmark results. Default: `10000`.

- `SEMPAL_PERF_WARN_P95_US_WHEEL`
Warning-only p95 latency threshold (microseconds) for wheel interaction
benchmark results. Default: `10000`.

- `SEMPAL_PERF_WARN_P95_US_MAP_PAN_PROXY`
Warning-only p95 latency threshold (microseconds) for map-pan-proxy interaction
benchmark results. Default: `12000`.

- `SEMPAL_PERF_WARN_P95_US_WAVEFORM`
Warning-only p95 latency threshold (microseconds) for waveform interaction
benchmark results. Default: `10000`.

- `SEMPAL_PERF_FAIL_P95_US_HOVER`
Optional hard-fail p95 latency threshold (microseconds) for hover interaction
benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_FILTER_CHURN`
Optional hard-fail p95 latency threshold (microseconds) for filter-only browser
churn benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_QUERY_CHURN`
Optional hard-fail p95 latency threshold (microseconds) for query-only browser
churn benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_SORT_CHURN`
Optional hard-fail p95 latency threshold (microseconds) for sort-only browser
churn benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_WHEEL`
Hard-fail p95 latency threshold (microseconds) for wheel interaction benchmark
results. Default: `30000` (can be overridden; set to a very high value to
effectively disable).

- `SEMPAL_PERF_FAIL_P95_US_FOCUS_PREVIEW`
Optional hard-fail p95 latency threshold (microseconds) for browser-focus
preview interaction benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_FOCUS_COMMIT`
Hard-fail p95 latency threshold (microseconds) for browser-focus commit
interaction benchmark results. Default: `100000` (can be overridden; set to a
very high value to effectively disable).

- `SEMPAL_PERF_FAIL_P95_US_WAVEFORM`
Optional hard-fail p95 latency threshold (microseconds) for waveform
interaction benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_MAP_PAN_PROXY`
Hard-fail p95 latency threshold (microseconds) for map-pan-proxy interaction
benchmark results. Default: `4000` (can be overridden; set to a very high value
to effectively disable).

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

## Wheel stability workflow overrides

- `SEMPAL_PERF_WHEEL_STABILITY_ROOT`
Artifact root used by `scripts/run_perf_wheel_stability.sh` for collected
wheel-latency evidence and summary outputs. Default:
`target/perf/wheel_stability`.

- `SEMPAL_PERF_WHEEL_STABILITY_WINDOWS`
Number of evidence windows collected in `collect` mode. Default: `7`.

- `SEMPAL_PERF_WHEEL_STABILITY_RUNS_PER_WINDOW`
Number of perf-guard runs executed per collected window. Default: `3`.

- `SEMPAL_PERF_WHEEL_STABILITY_REQUIRED_WINDOWS`
Number of most-recent windows required by `evaluate` mode before promotion can
be marked ready. Default: `7`.

- `SEMPAL_PERF_WHEEL_STABILITY_TARGET_P95_US`
Maximum median wheel p95 (microseconds) allowed per window for promotion
readiness. Default: `16000`.

- `SEMPAL_PERF_WHEEL_STABILITY_MAX_P95_SPREAD_US`
Maximum in-window wheel p95 spread (microseconds) allowed for readiness.
Default: `7000`.

- `SEMPAL_PERF_WHEEL_STABILITY_MAX_STDDEV_US`
Maximum median wheel standard deviation (microseconds) allowed per window.
Default: `6000`.

- `SEMPAL_PERF_WHEEL_STABILITY_MAX_OUTLIER_RATIO`
Maximum median wheel high-outlier ratio allowed per window (0.0-1.0).
Default: `0.35`.

- `SEMPAL_PERF_WHEEL_STABILITY_SUMMARY_OUT`
Output path for wheel-stability readiness summary JSON written by
`scripts/run_perf_wheel_stability.sh`. Default:
`target/perf/wheel_stability/wheel_stability_summary.json`.

- `SEMPAL_PERF_WHEEL_STABILITY_ENFORCE_READY`
When truthy (`1`, `true`, `yes`, `on`), `scripts/run_perf_wheel_stability.sh
evaluate` exits non-zero if readiness criteria are not met. Default: `0`.

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
