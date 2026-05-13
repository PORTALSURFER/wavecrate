//! Logging setup for the application.
//!
//! Initializes a global tracing subscriber that writes to both stdout and a
//! per-launch log file. Files are timestamped and kept to a bounded count to
//! avoid unbounded growth.

mod contract;
mod policy;

pub use contract::{
    ACTION_EVENT_TARGET, ActionDebugEvent, DB_EVENT_TARGET, DbDebugEvent, emit_action_debug_event,
    emit_db_debug_event,
};
pub use policy::{
    DEBUG_LOGGING_ARG, DEBUG_LOGGING_ENV_VAR, DEBUG_LOGGING_SHORT_ARG, DebugLoggingMode,
    DebugLoggingSettings,
};

use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    panic,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::SystemTime,
};

use time::{OffsetDateTime, UtcOffset, format_description::FormatItem, macros::format_description};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{Registry, fmt, prelude::*};

use crate::app_dirs;

/// Maximum number of log files to retain.
const MAX_LOG_FILES: usize = 10;
const LOG_FILE_PREFIX: &str = "wavecrate";

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static DEBUG_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Errors that may occur while initializing logging.
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    /// No platform-specific data directory could be resolved.
    #[error("No suitable data directory available for logs")]
    NoDataDir,
    /// Failed to create or access the log directory.
    #[error("Failed to prepare log directory {path}: {source}")]
    CreateDir {
        /// Log directory path.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to resolve the configured persistence profile for logs.
    #[error("Invalid log persistence profile '{profile}'")]
    InvalidProfile {
        /// Rejected profile name.
        profile: String,
    },
    /// Failed to enumerate existing log files for pruning.
    #[error("Failed to read log directory {path}: {source}")]
    ReadDir {
        /// Log directory path.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to remove an obsolete log file.
    #[error("Failed to remove old log file {path}: {source}")]
    RemoveFile {
        /// Log file path that failed to delete.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Failed to format a timestamp for the log filename.
    #[error("Failed to format log filename time: {0}")]
    FormatTime(time::error::Format),
    /// Failed to set the global tracing subscriber.
    #[error("Failed to install global tracing subscriber: {0}")]
    SetGlobal(tracing::subscriber::SetGlobalDefaultError),
    /// Failed to create the initial log file for this launch.
    #[error("Failed to create log file at {path}: {source}")]
    CreateLogFile {
        /// Log file path.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

/// Initialize tracing to write to stdout and a per-launch log file.
///
/// Subsequent calls are no-ops. Failures are returned so callers can degrade
/// gracefully without aborting startup.
pub fn init<I>(args: I) -> Result<(), LoggingError>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    if LOG_GUARD.get().is_some() {
        return Ok(());
    }

    let settings = DebugLoggingSettings::from_process(args);
    let log_dir = log_directory()?;
    let log_file_name = format_log_file_name(now_local_or_utc())?;
    let log_path = log_dir.join(&log_file_name);
    ensure_file_exists(&log_path)?;

    let file_appender = rolling::never(&log_dir, log_file_name);
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    prune_old_logs(&log_dir, MAX_LOG_FILES)?;

    let timer = build_timer();
    let env_filter = settings.env_filter();
    let stdout_layer = fmt::layer()
        .with_timer(timer.clone())
        .with_writer(std::io::stdout);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_timer(timer)
        .with_writer(file_writer);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer);
    tracing::subscriber::set_global_default(subscriber).map_err(LoggingError::SetGlobal)?;
    let _ = LOG_GUARD.set(guard);
    DEBUG_LOGGING_ENABLED.store(settings.mode().enabled(), Ordering::Relaxed);
    wavecrate_library::diagnostics::set_debug_logging_enabled(settings.mode().enabled());

    tracing::info!(
        log_path = %log_path.display(),
        debug_logging_mode = settings.mode().as_str(),
        debug_logging_launch_arg = settings.enabled_by_launch_arg(),
        debug_logging_filter_source = settings.filter_source(),
        debug_logging_filter = settings.filter_description(),
        "Logging initialized"
    );
    if let Some(invalid_value) = settings.invalid_debug_value() {
        tracing::warn!(
            env_var = DEBUG_LOGGING_ENV_VAR,
            invalid_value,
            "Ignoring invalid debug logging flag; expected one of 1/0/true/false/on/off/yes/no"
        );
    }
    Ok(())
}

/// Returns `true` when the Wavecrate-owned debug logging mode is enabled.
///
/// Rich action/database diagnostics should use this gate instead of treating a
/// broad `RUST_LOG` override as product intent.
pub fn debug_logging_enabled() -> bool {
    DEBUG_LOGGING_ENABLED.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn set_debug_logging_enabled_for_tests(enabled: bool) {
    DEBUG_LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Install a panic hook that writes panic context and backtrace to logs.
///
/// The hook preserves the previous panic handler so default panic rendering is still
/// emitted for diagnostics outside tracing consumers.
pub fn install_panic_hook() {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let current_thread = std::thread::current();
        let thread = current_thread.name().unwrap_or("<unnamed>");
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<String>()
                    .map(String::as_str)
            })
            .unwrap_or("<non-string panic payload>");
        match panic_info.location() {
            Some(location) => tracing::error!(
                "panic in thread={thread} at {file}:{line}:{column}: {payload}",
                file = location.file(),
                line = location.line(),
                column = location.column()
            ),
            None => tracing::error!("panic in thread={thread}: {payload}"),
        }
        tracing::error!("panic backtrace:\n{:?}", Backtrace::force_capture());
        previous_hook(panic_info);
    }));
}

fn log_directory() -> Result<PathBuf, LoggingError> {
    app_dirs::logs_dir().map_err(map_app_dir_error)
}

fn ensure_file_exists(path: &Path) -> Result<(), LoggingError> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|_| ())
        .map_err(|source| LoggingError::CreateLogFile {
            path: path.to_path_buf(),
            source,
        })
}

fn prune_old_logs(dir: &Path, max_files: usize) -> Result<(), LoggingError> {
    let mut entries = fs::read_dir(dir)
        .map_err(|source| LoggingError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("log"))
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, entry.path())
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|(modified, _)| *modified);
    while entries.len() > max_files {
        if let Some((_, path)) = entries.first() {
            fs::remove_file(path).map_err(|source| LoggingError::RemoveFile {
                path: path.to_path_buf(),
                source,
            })?;
        }
        entries.remove(0);
    }
    Ok(())
}

fn format_log_file_name(now: OffsetDateTime) -> Result<String, LoggingError> {
    const NAME_FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");
    let name = now.format(NAME_FORMAT).map_err(LoggingError::FormatTime)?;
    Ok(format!("{LOG_FILE_PREFIX}_{name}.log"))
}

fn build_timer() -> fmt::time::OffsetTime<time::format_description::BorrowedFormatItem<'static>> {
    const DISPLAY_FORMAT: &[FormatItem<'static>] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    fmt::time::OffsetTime::new(offset, DISPLAY_FORMAT.into())
}

fn now_local_or_utc() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn map_app_dir_error(error: app_dirs::AppDirError) -> LoggingError {
    match error {
        app_dirs::AppDirError::NoBaseDir => LoggingError::NoDataDir,
        app_dirs::AppDirError::CreateDir { path, source } => {
            LoggingError::CreateDir { path, source }
        }
        app_dirs::AppDirError::InvalidProfileName { profile } => {
            LoggingError::InvalidProfile { profile }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};
    use tempfile::tempdir;

    #[test]
    fn log_filename_has_timestamp_and_prefix() {
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let name = format_log_file_name(fixed).unwrap();
        assert_eq!(name, "wavecrate_2023-11-14_22-13-20.log");
    }

    #[test]
    fn prune_removes_oldest_files_beyond_limit() {
        let dir = tempdir().unwrap();
        for idx in 0..12 {
            let path = dir.path().join(format!("wavecrate_{idx}.log"));
            ensure_file_exists(&path).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        prune_old_logs(dir.path(), 10).unwrap();
        let remaining = fs::read_dir(dir.path())
            .unwrap()
            .filter(|entry| {
                entry.as_ref().ok().map(|e| e.path()).is_some_and(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "log")
                        .unwrap_or(false)
                })
            })
            .count();
        assert_eq!(remaining, 10);
    }

    #[test]
    fn prune_keeps_newest_log_files() {
        let dir = tempdir().unwrap();
        for idx in 0..12 {
            let path = dir.path().join(format!("wavecrate_{idx}.log"));
            ensure_file_exists(&path).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        prune_old_logs(dir.path(), 10).unwrap();

        let mut remaining = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".log"))
            .collect::<Vec<_>>();
        remaining.sort();

        assert_eq!(
            remaining,
            vec![
                "wavecrate_10.log".to_string(),
                "wavecrate_11.log".to_string(),
                "wavecrate_2.log".to_string(),
                "wavecrate_3.log".to_string(),
                "wavecrate_4.log".to_string(),
                "wavecrate_5.log".to_string(),
                "wavecrate_6.log".to_string(),
                "wavecrate_7.log".to_string(),
                "wavecrate_8.log".to_string(),
                "wavecrate_9.log".to_string(),
            ]
        );
    }

    #[test]
    fn prune_ignores_non_log_files() {
        let dir = tempdir().unwrap();
        for idx in 0..12 {
            let path = dir.path().join(format!("wavecrate_{idx}.log"));
            ensure_file_exists(&path).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        let non_log_path = dir.path().join("keep.txt");
        ensure_file_exists(&non_log_path).unwrap();

        prune_old_logs(dir.path(), 10).unwrap();

        assert!(non_log_path.exists());
    }
}
