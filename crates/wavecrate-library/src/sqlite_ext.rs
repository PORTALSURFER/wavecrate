//! Optional SQLite extension loader for accelerated vector operations.
//!
//! By default, Wavecrate runs entirely on built-in SQLite capabilities.
//! If `WAVECRATE_SQLITE_EXT` points at a loadable extension, Wavecrate will attempt
//! to load it and continue with a safe fallback if loading fails. Loading is
//! gated by `WAVECRATE_SQLITE_EXT_ENABLE` and restricted to app-owned directories
//! unless `WAVECRATE_SQLITE_EXT_UNSAFE` is explicitly set and the build enables
//! the `sqlite-ext-unsafe` feature. The allowlisted directory lives at
//! `<app_root>/sqlite_extensions`.

use rusqlite::Connection;
use tracing::warn;

mod loader;
mod policy;

pub use policy::ExtensionLoadOutcome;
use policy::ExtensionPolicyDecision;

/// Environment variable pointing at a loadable SQLite extension (.so/.dll/.dylib).
pub const SQLITE_EXT_ENV: &str = "WAVECRATE_SQLITE_EXT";

/// Environment variable that must be set to enable loading `WAVECRATE_SQLITE_EXT`.
pub const SQLITE_EXT_ENABLE_ENV: &str = "WAVECRATE_SQLITE_EXT_ENABLE";

/// Environment variable that bypasses extension safety checks and allowlist
/// enforcement when set. This is ignored unless the `sqlite-ext-unsafe` cargo
/// feature is enabled at build time.
pub const SQLITE_EXT_UNSAFE_ENV: &str = "WAVECRATE_SQLITE_EXT_UNSAFE";

const SQLITE_EXT_DIR_NAME: &str = "sqlite_extensions";

/// Attempt to load the optional SQLite extension specified by `WAVECRATE_SQLITE_EXT`.
///
/// This is a best-effort operation:
/// - If the env var is unset, this returns [`ExtensionLoadOutcome::NotConfigured`].
/// - If `WAVECRATE_SQLITE_EXT_ENABLE` is not set, this returns a structured
///   disabled outcome and logs the same warning as the legacy loader.
/// - The extension must live under the app-owned `sqlite_extensions` directory unless
///   `WAVECRATE_SQLITE_EXT_UNSAFE` is set. In unsafe mode, the path is resolved as provided
///   (absolute or relative to the current working directory).
/// - If policy blocks or skips loading, this returns a structured non-error outcome.
/// - If SQLite loading itself fails, the `rusqlite` error is returned to the caller
///   so it can be logged and ignored by optional-extension callers.
pub fn try_load_optional_extension(
    conn: &Connection,
) -> Result<ExtensionLoadOutcome, rusqlite::Error> {
    match policy::extension_policy_from_env() {
        ExtensionPolicyDecision::Skip(outcome) => {
            outcome.log_if_needed();
            Ok(outcome)
        }
        ExtensionPolicyDecision::Load(plan) => {
            if plan.unsafe_mode() {
                warn!(
                    path = %plan.path().display(),
                    "{SQLITE_EXT_UNSAFE_ENV} set; bypassing SQLite extension safety checks."
                );
            }
            loader::load_extension(conn, plan.path())?;
            Ok(ExtensionLoadOutcome::Loaded {
                path: plan.path().to_path_buf(),
            })
        }
    }
}
