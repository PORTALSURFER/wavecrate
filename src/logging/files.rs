//! Log file path resolution and maintenance helpers.

use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    time::SystemTime,
};

use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

use super::LoggingError;
use crate::app_dirs;

/// Maximum number of log files to retain.
const MAX_LOG_FILES: usize = 10;
const LOG_FILE_PREFIX: &str = "wavecrate";

/// Explicit log path projection for the active persistence profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LogProfilePaths {
    /// Active application root for the selected profile.
    pub(crate) app_root: PathBuf,
    /// Log directory under the active application root.
    pub(crate) logs_dir: PathBuf,
}

/// Prepared per-launch log file details consumed by runtime subscriber setup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LaunchLogFile {
    /// Directory that contains Wavecrate log files.
    pub(super) dir: PathBuf,
    /// File name used by the rolling appender.
    pub(super) file_name: String,
    /// Absolute path to the file for startup diagnostics.
    pub(super) path: PathBuf,
}

/// Resolve the current profile/app-root/log-dir paths without installing logging.
pub(crate) fn resolve_log_profile_paths() -> Result<LogProfilePaths, LoggingError> {
    let app_root = app_dirs::app_root_dir().map_err(map_app_dir_error)?;
    let logs_dir = app_dirs::logs_dir().map_err(map_app_dir_error)?;
    Ok(LogProfilePaths { app_root, logs_dir })
}

/// Prepare the per-launch log file and prune old log files.
pub(super) fn prepare_launch_log_file() -> Result<LaunchLogFile, LoggingError> {
    let log_dir = resolve_log_profile_paths()?.logs_dir;
    let log_file_name = format_log_file_name(now_local_or_utc())?;
    let log_path = log_dir.join(&log_file_name);
    ensure_file_exists(&log_path)?;
    prune_old_logs(&log_dir, MAX_LOG_FILES)?;
    Ok(LaunchLogFile {
        dir: log_dir,
        file_name: log_file_name,
        path: log_path,
    })
}

/// Return the newest `.log` file under one log directory.
pub(crate) fn newest_log_file(dir: &Path) -> Result<Option<PathBuf>, LoggingError> {
    let mut entries = log_files_by_modified_time(dir)?;
    entries.sort_by_key(|(modified, _)| *modified);
    Ok(entries.pop().map(|(_, path)| path))
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
    let mut entries = log_files_by_modified_time(dir)?;
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

fn log_files_by_modified_time(dir: &Path) -> Result<Vec<(SystemTime, PathBuf)>, LoggingError> {
    Ok(fs::read_dir(dir)
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
        .collect())
}

fn format_log_file_name(now: OffsetDateTime) -> Result<String, LoggingError> {
    const NAME_FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");
    let name = now.format(NAME_FORMAT).map_err(LoggingError::FormatTime)?;
    Ok(format!("{LOG_FILE_PREFIX}_{name}.log"))
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
    use crate::app_dirs::{ConfigBaseGuard, PersistenceProfileGuard};
    use std::{path::Path, thread, time::Duration};
    use tempfile::tempdir;

    fn explicit_persistence_env_present() -> bool {
        std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
            || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
    }

    #[test]
    fn log_filename_has_timestamp_and_prefix() {
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let name = format_log_file_name(fixed).unwrap();
        assert_eq!(name, "wavecrate_2023-11-14_22-13-20.log");
    }

    #[test]
    fn log_profile_paths_make_live_root_explicit() {
        if explicit_persistence_env_present() {
            return;
        }
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::live();

        let paths = resolve_log_profile_paths().unwrap();

        assert_eq!(paths.app_root, base.path().join(".wavecrate"));
        assert_eq!(paths.logs_dir, paths.app_root.join("logs"));
    }

    #[test]
    fn log_profile_paths_make_sandbox_profile_explicit() {
        if explicit_persistence_env_present() {
            return;
        }
        let base = tempdir().unwrap();
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::sandbox();

        let paths = resolve_log_profile_paths().unwrap();

        assert_eq!(
            paths.app_root,
            base.path()
                .join(".wavecrate")
                .join("profiles")
                .join("sandbox")
        );
        assert_eq!(paths.logs_dir, paths.app_root.join("logs"));
    }

    #[test]
    fn prepare_launch_log_file_creates_log_and_prunes_old_files() {
        let dir = tempdir().unwrap();
        for idx in 0..12 {
            let path = dir.path().join(format!("wavecrate_{idx}.log"));
            ensure_file_exists(&path).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        prune_old_logs(dir.path(), 10).unwrap();

        let remaining = count_logs(dir.path());
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

    #[test]
    fn newest_log_file_returns_most_recent_log_only() {
        let dir = tempdir().unwrap();
        let older = dir.path().join("older.log");
        ensure_file_exists(&older).unwrap();
        thread::sleep(Duration::from_millis(10));
        let newer = dir.path().join("newer.log");
        ensure_file_exists(&newer).unwrap();
        thread::sleep(Duration::from_millis(10));
        ensure_file_exists(&dir.path().join("ignored.txt")).unwrap();

        assert_eq!(newest_log_file(dir.path()).unwrap(), Some(newer));
    }

    fn count_logs(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter(|entry| {
                entry.as_ref().ok().map(|e| e.path()).is_some_and(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext == "log")
                })
            })
            .count()
    }
}
