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
Overrides the build tool used by `scripts/internal/release/build_release_zip.sh` (defaults to
`cargo`). CI sets this to `cross` for cross builds.

- `SEMPAL_CHECKSUMS_ED25519_KEY`
CI secret used by `.github/workflows/release-build.yml` to sign release checksum
files. This is expected to be an Ed25519 private key in PEM form that OpenSSL
can use for `pkeyutl -sign`.

- `SEMPAL_DISABLE_SCCACHE`
When set to `1`, repo Cargo helper scripts skip auto-configuring `sccache`
even when it is installed. On PowerShell lanes, this also clears an inherited
`RUSTC_WRAPPER=sccache` for the current script run so the wrappers fall back to
direct `rustc`. This is useful when comparing uncached timings or debugging
wrapper-related issues. Default: unset (`sccache` is auto-used when available
and its wrapper probe passes).

- `SEMPAL_WINDOWS_SIGN_PFX_BASE64`
CI secret used by `.github/workflows/release-build.yml` to Authenticode-sign
Windows executables. This value is expected to be a base64-encoded `.pfx`
certificate bundle.

- `SEMPAL_WINDOWS_SIGN_PFX_PASSWORD`
Password for `SEMPAL_WINDOWS_SIGN_PFX_BASE64`, consumed by the Windows signing
step in `.github/workflows/release-build.yml`.

## Agent workflow

- `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK_INSTALL`
Set to `1` to skip automatic preflight hook installation in bootstrap.
Hook execution for existing hook setups is separately controlled by
`SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1`.

- `SEMPAL_DEAD_SWEEP_STRICT`
When set to `1`/`true`/`yes`/`on`, `scripts/check.sh dead-deps`
exits non-zero when it detects unused dependency/dead-code findings. Default:
advisory mode (`0`).

- `SEMPAL_DEAD_SWEEP_RUN_UDEPS`
When set to `1`/`true`/`yes`/`on`,
`scripts/check.sh dead-deps` also runs `cargo udeps` using
`SEMPAL_DEAD_SWEEP_UDEPS_TOOLCHAIN` (or its default).

- `SEMPAL_DEAD_SWEEP_INSTALL_MISSING`
When set to `1`/`true`/`yes`/`on`,
`scripts/check.sh dead-deps` attempts to install missing
`cargo-machete`/`cargo-udeps` tools before running checks.

- `SEMPAL_DEAD_SWEEP_UDEPS_TOOLCHAIN`
Rust toolchain string passed to `cargo +<toolchain> udeps` by
`scripts/check.sh dead-deps`. Default: `nightly`.

- `SEMPAL_DEAD_SWEEP_REPORT_PATH`
Optional output path for `scripts/check.sh dead-deps` to write
the full advisory sweep report.

## Paths and directories

- `SEMPAL_CONFIG_HOME`
Overrides the OS config base directory used for app files. Sempal still creates
and uses a `.sempal` directory under this base.

Example: `SEMPAL_CONFIG_HOME=/tmp` causes logs to be written under
`/tmp/.sempal/logs` on Unix-like platforms.

- `SEMPAL_CONFIG_PROFILE`
Selects one named non-live persistence profile under the resolved app root.
`SEMPAL_CONFIG_PROFILE=gui-test` writes config, logs, and `library.db` under
`<config-base>/.sempal/profiles/gui-test/`. Set `SEMPAL_CONFIG_PROFILE=live`
to force the real live app root explicitly. Test executables default to the
named `automated-tests` profile when neither `SEMPAL_CONFIG_PROFILE` nor
`SEMPAL_CONFIG_HOME` is set.
Use `sandbox` for manual QA/sandboxed runs and `automated-tests` for automated
validation. `scripts/run.* sandbox` now sets `SEMPAL_CONFIG_PROFILE=sandbox`
explicitly so sandbox runs are first-class non-live profiles, not just
alternate config-home directories.

### Persistence profile quick reference

- `live`
  - app root: `<config-base>/.sempal/`
  - use for: normal user-facing startup and release/manual runs
- `sandbox`
  - app root: `<config-base>/.sempal/profiles/sandbox/`
  - use for: deliberate manual QA runs that must not touch the live profile
- `automated-tests`
  - app root: `<config-base>/.sempal/profiles/automated-tests/`
  - use for: tests, GUI contract runs, and scripted validation

The practical rule is simple: automated validation should prove behavior
through isolated fixture roots and the `automated-tests` profile, while a plain
`cargo run --release` with no overrides should still open the intended live
source registry afterward.

### What writes where (important for safe local runs)

- App config + logs:
  - Live runs write under `<config-base>/.sempal/`.
  - Named non-live profiles write under `<config-base>/.sempal/profiles/<name>/`.
  - `scripts/run.* sandbox` sets both `SEMPAL_CONFIG_HOME` and `SEMPAL_CONFIG_PROFILE=sandbox` so the run does **not** touch your real user profile config/log directories.
- Per-source-folder database side effect:
  - If you point Sempal at a folder, it may create or update a `.sempal_samples.db` in that source folder.
  - `SEMPAL_CONFIG_HOME` does **not** relocate these per-folder DB files.
  - `scripts/run.sh sandbox` and `scripts/run.ps1 sandbox` set
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

- `SEMPAL_GUI_TEST_MODE`
Enables deterministic GUI test mode for the main app binary.

- `SEMPAL_GUI_TEST_FIXTURE`
Selects the GUI fixture used in GUI test mode. The default is
`isolated-startup`, which keeps automated runs on a dedicated non-live
profile. Use `live` only when you explicitly want GUI validation to exercise
the real persisted startup profile. The legacy `default` tag is accepted only
 as an alias for `isolated-startup`.

## Logging

- `--log` / `-log`
Preferred Sempal-owned opt-in switch for richer per-launch debug diagnostics in
release or manual debugging runs.

Semantics:
- when absent, launch logs keep the normal always-on baseline (`info` level plus
  warnings/errors) and richer action/DB reconstruction events stay disabled
- when present and `RUST_LOG` is unset, Sempal installs the default filter
  `sempal=debug,info` so Sempal-owned debug events are visible without turning
  third-party crates up to `debug`
- when present and `RUST_LOG` is also set, `RUST_LOG` still owns the final
  filter expression, but Sempal still treats the run as debug mode for
  diagnostics that explicitly gate on the Sempal-owned debug feature
- on Windows release builds, the same flag also preserves the existing console
  attach behavior for interactive debugging

Structured debug event contract for later instrumentation:
- action events must use stable `action`, `pane`, `source`, `outcome`,
  `elapsed_ms`, and `error` fields
- DB events must use stable `operation`, `source`, `outcome`, `elapsed_ms`, and
  `error` fields
- use concise, sanitized identifiers; prefer stable action/operation names over
  free-form prose

Never log:
- secrets, tokens, auth headers, credentials, encryption keys, or session data
- raw SQL with interpolated private values
- large unredacted user-authored payloads or other sensitive content that is not
  required to diagnose the seam

- `SEMPAL_DEBUG_LOGGING`
Optional compatibility env override for non-interactive launch environments.
Accepted truthy values: `1`, `true`, `yes`, `on`. Accepted falsy values: `0`,
`false`, `no`, `off`.

- `RUST_LOG`
Controls logging verbosity via `tracing_subscriber::EnvFilter`. When unset,
Sempal defaults to `info`, or to `sempal=debug,info` when the Sempal-owned
debug feature is enabled via `--log`, `-log`, or `SEMPAL_DEBUG_LOGGING`.
`RUST_LOG` overrides that default filter when it is set.

### Debug log workflow

- local release debugging:
  - live profile:
    `cargo run --release -- --log`
  - sandbox/manual QA profile:
    `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 sandbox -- --log`
- newest log retrieval:
  - live profile:
    `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs`
  - sandbox/manual QA profile:
    `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs -Sandbox`
- bug-report collection:
  - live profile:
    `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 bug-bundle`
  - sandbox/manual QA profile:
    `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 bug-bundle -Sandbox`
- what the log captures automatically:
  - startup metadata, warnings/errors, and the structured Sempal-owned debug
    action/DB events emitted during the reproduced run
- what still requires manual reproduction:
  - you still need to rerun the failing scenario with `--log`; the file records
    what happened during that run, but it does not retroactively reconstruct an
    earlier session that was not logged

## Headless audio

- `ALSA_CONFIG_PATH`
When set, ALSA uses the provided config file path instead of system defaults.
On headless Linux runs, `scripts/ci.sh local` and `scripts/perf.sh guard`
set this automatically to `scripts/internal/alsa_headless.conf` unless already defined.
This routes default playback probing to a dummy sink to reduce `ALSA lib`
warning noise in test/bench logs.

## UI and runtime profiling

- `SEMPAL_BROWSER_SEARCH_OFFLOAD_THRESHOLD`
Wav-entry threshold used by browser search to switch from synchronous layout
path recompute to async search jobs. Default: `5000`.

- `RADIANT_NATIVE_RENDER_PROFILE`
Enables Radiant native render profiling logs when the build enables the
`gui-performance` cargo feature. Accepted values include `1`, `true`, `on`,
and `yes` (case variants are accepted in the current implementation).

- `RADIANT_NATIVE_STARTUP_PROFILE`
Enables one-line Radiant native startup timing summaries after first present
plus deferred model refresh. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive). Intended for startup-focused profiling workflows.

- `RADIANT_NATIVE_INCREMENTAL_FRAME_PIPELINE`
Enables bridge-driven dirty-segment gating for native static frame rebuilds in
the Radiant `winit + vello` runtime. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive).

- `SEMPAL_NATIVE_BRIDGE_PROFILE`
Enables native bridge profiling logs when the build enables the
`native-bridge-metrics` cargo feature. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive).

- `SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT`
Enables runtime validation that the cached native-bridge projection-key
snapshot still matches a freshly rebuilt key before projection. This is only
available when the build enables the `native-bridge-metrics` cargo feature.
Accepted values: `1`, `true`, `on`, `yes` (case-insensitive).

- `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW`
Controls whether waveform overlay preview actions (cursor + selection-range
updates) apply immediately in the native bridge instead of waiting for queue
flush boundaries. Accepted values: `1`, `true`, `on`, `yes`
(case-insensitive). Default: enabled (`true`).

## Performance guard benchmark overrides

- `SEMPAL_PERF_GUARD_OUT`
Output path used by `scripts/perf.sh guard` for the benchmark JSON report.
Default: `target/perf/bench.json`.

- `SEMPAL_PERF_GUARD_GUI_ROWS`
Synthetic GUI row count passed to the `sempal-bench` benchmark CLI for the
guard run.
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
Number of full `sempal-bench` benchmark CLI runs executed by
`scripts/perf.sh guard`. When greater than `1`, the guard reports median
percentiles across runs and the p95 spread across runs. Default: `1`.

- `SEMPAL_PERF_GUARD_STARTUP_PROFILE`
When set to `1`/`true`/`on`/`yes`, `scripts/perf.sh guard` also captures
native startup timing logs by launching `sempal` under timeout and parsing
`RADIANT_NATIVE_STARTUP_PROFILE` output. Default: disabled (`0`).

- `SEMPAL_PERF_GUARD_STARTUP_TIMEOUT_SECS`
Timeout (seconds) used for each startup-profile capture run when startup
profiling is enabled. Default: `6`.

- `SEMPAL_PERF_GUARD_STARTUP_MIN_VALID_RUNS`
Minimum number of startup profile captures that must include valid
`[native-vello-startup]` lines for startup summary confidence checks.
Default: `3` when `SEMPAL_PERF_GUARD_RUNS >= 3`, otherwise `1`.

- `SEMPAL_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS`
When set to `1`/`true`/`on`/`yes`, startup summary parsing fails if valid runs
fall below `SEMPAL_PERF_GUARD_STARTUP_MIN_VALID_RUNS`. Default: disabled (`0`).

- `SEMPAL_PERF_GUARD_STARTUP_SUMMARY_OUT`
Output path for the startup timing summary JSON generated by
`scripts/internal/perf/perf_startup_summary.py` when startup profiling is enabled.
Default: `<SEMPAL_PERF_GUARD_OUT>.startup_summary.json`.

- `SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT`
Optional output env file path for startup threshold locking. When set and
startup profiling succeeds, `scripts/perf.sh guard` writes startup threshold
assignments to this file using `scripts/internal/perf/perf_startup_lock_thresholds.py`.

- `SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_IN`
Optional startup threshold lock-file input path sourced by
`scripts/perf.sh guard` before threshold parsing. Defaults to the tracked
lock file at `scripts/internal/perf/locks/startup_thresholds.env`. Set to an empty value
to disable auto-loading.

- `SEMPAL_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS`
Minimum valid startup-profile run count required before writing
`SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT`. Defaults to
`SEMPAL_PERF_GUARD_STARTUP_MIN_VALID_RUNS`.

- `SEMPAL_PERF_GUARD_FRAME_QUALITY_LOCK_ENV_OUT`
Optional output env file path for frame-quality threshold locking. When set,
`scripts/perf.sh guard` writes calibrated frame-jank and missed-present
threshold assignments to this path using
`scripts/internal/perf/perf_frame_quality_lock_thresholds.py`.

- `SEMPAL_PERF_GUARD_FRAME_QUALITY_LOCK_MIN_RUNS`
Minimum benchmark-report run count required before writing
`SEMPAL_PERF_GUARD_FRAME_QUALITY_LOCK_ENV_OUT`. Defaults to `3` when
`SEMPAL_PERF_GUARD_RUNS >= 3`, otherwise `1`.

- `SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_MS`
Warning threshold (milliseconds) for median startup `first_present_ms` in
startup profile summaries. Default: `800`.

- `SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_MS`
Optional hard-fail threshold (milliseconds) for median startup
`first_present_ms` in startup profile summaries. Unset by default.

- `SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS`
Optional warning threshold (milliseconds) for startup `first_present_ms` spread
across captured runs. Unset by default.

- `SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS`
Optional hard-fail threshold (milliseconds) for startup
`first_present_ms` spread across captured runs. Unset by default.

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

- `SEMPAL_PERF_WARN_P95_US_WAVEFORM_ADJACENT`
Warning-only p95 latency threshold (microseconds) for adjacent waveform
pan/zoom interaction benchmark results. Default: `12000`.

- `SEMPAL_PERF_WARN_P95_US_VOLUME`
Warning-only p95 latency threshold (microseconds) for continuous volume-drag
interaction benchmark results. Default: `8000`.

- `SEMPAL_PERF_WARN_FRAME_JANK_RATIO`
Warning threshold (ratio `0.0..=1.0`) for per-scenario frame-jank proxy share
reported by `scripts/perf.sh guard` from benchmark latency samples above
`frame_budget_us` (`16667us`). Default: `0.10`.

- `SEMPAL_PERF_FAIL_FRAME_JANK_RATIO`
Optional hard-fail threshold (ratio `0.0..=1.0`) for per-scenario frame-jank
proxy share. Unset by default.

- `SEMPAL_PERF_WARN_MISSED_PRESENT_PROXY_RATIO`
Warning threshold (ratio `0.0..=1.0`) for per-scenario missed-present proxy
share reported by `scripts/perf.sh guard` from benchmark latency samples
above `2 * frame_budget_us` (`33334us`). Default: `0.05`.

- `SEMPAL_PERF_FAIL_MISSED_PRESENT_PROXY_RATIO`
Optional hard-fail threshold (ratio `0.0..=1.0`) for per-scenario
missed-present proxy share. Unset by default.

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

- `SEMPAL_PERF_FAIL_P95_US_WAVEFORM_ADJACENT`
Optional hard-fail p95 latency threshold (microseconds) for adjacent waveform
pan/zoom interaction benchmark results. Unset by default.

- `SEMPAL_PERF_FAIL_P95_US_VOLUME`
Optional hard-fail p95 latency threshold (microseconds) for continuous
volume-drag interaction benchmark results. Unset by default.

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
Test-only audio-device override. When running tests, set
`SEMPAL_TEST_AUDIO_OUTPUT=1` (or `true`/`yes`/`on`) to exercise real audio
output devices. By default this is disabled so tests avoid opening host audio
devices, which keeps headless CI runs deterministic and reduces ALSA/driver
noise.

- `SEMPAL_TRANSIENT_DEBUG`
When set (to any value), enables transient detection debug logs.

- `SEMPAL_PANNS_GOLDEN_PATH`
Path to the golden log-mel JSON used by the golden regression tests (typically
set by `scripts/check.sh golden-tests` / `scripts/check.ps1 golden-tests`).

- `SEMPAL_PANNS_EMBED_GOLDEN_PATH`
Path to the golden embedding JSON used by the golden regression tests (typically
set by `scripts/check.sh golden-tests` / `scripts/check.ps1 golden-tests`).

## Wheel stability workflow overrides

- `SEMPAL_PERF_WHEEL_STABILITY_ROOT`
Artifact root used by `scripts/perf.sh wheel-stability` for collected
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
`scripts/perf.sh wheel-stability`. Default:
`target/perf/wheel_stability/wheel_stability_summary.json`.

- `SEMPAL_PERF_WHEEL_STABILITY_ENFORCE_READY`
When truthy (`1`, `true`, `yes`, `on`), `scripts/perf.sh wheel-stability
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
