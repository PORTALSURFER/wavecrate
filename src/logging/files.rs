//! Log file path resolution and maintenance helpers.

use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

use super::LoggingError;
use crate::app_dirs;

/// Maximum number of log files to retain.
const MAX_LOG_FILES: usize = 10;
const LOG_FILE_PREFIX: &str = "wavecrate";

// OPT-1797 stages the size boundary for the later runtime integration slice.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "OPT-1801 will wire the size boundary into logging::init"
    )
)]
pub(super) mod size_capped;

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
    /// The reserved run identity and next segment sequence for the runtime integration slice.
    pub(super) run: LogSegmentRun,
}

/// One launch's unique namespace for subsequent create-new log segments.
///
/// Names are `wavecrate_<timestamp>_<16-hex run ordinal>_<16-hex sequence>.log`.
/// The run ordinal advances across same-second launches; `create_new` resolves a concurrent
/// launch race. Fixed-width suffixes order rotations and relaunches when mtimes tie.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LogSegmentRun {
    dir: PathBuf,
    timestamp: String,
    run_ordinal: u64,
    next_sequence: u64,
}

impl LogSegmentRun {
    /// Create the next segment without appending to or replacing an existing object.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "OPT-1801 will open rotated segments")
    )]
    pub(super) fn open_next(&mut self) -> io::Result<(PathBuf, File)> {
        let following = self.next_sequence.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::AlreadyExists, "log sequence exhausted")
        })?;
        let file_name =
            format_segment_file_name(&self.timestamp, self.run_ordinal, self.next_sequence);
        let path = self.dir.join(file_name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        self.next_sequence = following;
        Ok((path, file))
    }
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
    let (run, log_path) = start_log_run(&log_dir, now_local_or_utc())?;
    let log_file_name = log_path
        .file_name()
        .expect("created log segment has a filename")
        .to_string_lossy()
        .into_owned();
    prune_old_logs(&log_dir, MAX_LOG_FILES, Some(&log_path))?;
    Ok(LaunchLogFile {
        dir: log_dir,
        file_name: log_file_name,
        path: log_path,
        run,
    })
}

/// Return the newest `.log` file under one log directory.
pub(crate) fn newest_log_file(dir: &Path) -> Result<Option<PathBuf>, LoggingError> {
    let mut entries = log_files_by_modified_time(dir)?;
    sort_log_entries(&mut entries);
    Ok(entries.pop().map(|(_, path)| path))
}

#[cfg(test)]
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

fn prune_old_logs(
    dir: &Path,
    max_files: usize,
    protected: Option<&Path>,
) -> Result<(), LoggingError> {
    let mut entries = log_files_by_modified_time(dir)?;
    sort_log_entries(&mut entries);
    while entries.len() > max_files {
        let Some(oldest_unprotected) = entries
            .iter()
            .position(|(_, path)| protected != Some(path.as_path()))
        else {
            break;
        };
        let path = &entries[oldest_unprotected].1;
        fs::remove_file(path).map_err(|source| LoggingError::RemoveFile {
            path: path.to_path_buf(),
            source,
        })?;
        entries.remove(oldest_unprotected);
    }
    Ok(())
}

fn sort_log_entries(entries: &mut [(SystemTime, PathBuf)]) {
    entries.sort_by(|(time_a, path_a), (time_b, path_b)| {
        time_a.cmp(time_b).then_with(|| path_a.cmp(path_b))
    });
}

fn log_files_by_modified_time(dir: &Path) -> Result<Vec<(SystemTime, PathBuf)>, LoggingError> {
    Ok(fs::read_dir(dir)
        .map_err(|source| LoggingError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter(|entry| {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return false;
            };
            name.starts_with(LOG_FILE_PREFIX) && name.ends_with(".log")
        })
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, entry.path())
        })
        .collect())
}

fn format_log_timestamp(now: OffsetDateTime) -> Result<String, LoggingError> {
    const NAME_FORMAT: &[FormatItem<'_>] =
        format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");
    now.format(NAME_FORMAT).map_err(LoggingError::FormatTime)
}

fn format_segment_file_name(timestamp: &str, run_ordinal: u64, sequence: u64) -> String {
    format!("{LOG_FILE_PREFIX}_{timestamp}_{run_ordinal:016x}_{sequence:016x}.log")
}

fn parse_run_ordinal(name: &str, timestamp: &str) -> Option<u64> {
    let prefix = format!("{LOG_FILE_PREFIX}_{timestamp}_");
    let suffix = name.strip_prefix(&prefix)?.strip_suffix(".log")?;
    let (run, sequence) = suffix.split_once('_')?;
    if run.len() != 16 || sequence.len() != 16 {
        return None;
    }
    let run = u64::from_str_radix(run, 16).ok()?;
    u64::from_str_radix(sequence, 16).ok()?;
    Some(run)
}

fn start_log_run(
    dir: &Path,
    now: OffsetDateTime,
) -> Result<(LogSegmentRun, PathBuf), LoggingError> {
    let timestamp = format_log_timestamp(now)?;
    let mut highest_run = None::<u64>;
    for entry in fs::read_dir(dir).map_err(|source| LoggingError::ReadDir {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| LoggingError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        if let Some(run) = entry
            .file_name()
            .to_str()
            .and_then(|name| parse_run_ordinal(name, &timestamp))
        {
            highest_run = Some(highest_run.map_or(run, |highest| highest.max(run)));
        }
    }

    let mut run_ordinal = match highest_run {
        Some(highest) => highest
            .checked_add(1)
            .ok_or_else(|| LoggingError::CreateLogFile {
                path: dir.to_path_buf(),
                source: io::Error::new(io::ErrorKind::AlreadyExists, "log run ordinals exhausted"),
            })?,
        None => 0,
    };
    loop {
        let path = dir.join(format_segment_file_name(&timestamp, run_ordinal, 0));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                return Ok((
                    LogSegmentRun {
                        dir: dir.to_path_buf(),
                        timestamp,
                        run_ordinal,
                        next_sequence: 1,
                    },
                    path,
                ));
            }
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                run_ordinal =
                    run_ordinal
                        .checked_add(1)
                        .ok_or_else(|| LoggingError::CreateLogFile {
                            path: path.clone(),
                            source: io::Error::new(
                                io::ErrorKind::AlreadyExists,
                                "log run ordinals exhausted",
                            ),
                        })?;
            }
            Err(source) => return Err(LoggingError::CreateLogFile { path, source }),
        }
    }
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
    use filetime::{FileTime, set_file_mtime};
    use std::{path::Path, thread, time::Duration};
    use tempfile::tempdir;

    fn explicit_persistence_env_present() -> bool {
        std::env::var_os("WAVECRATE_CONFIG_HOME").is_some()
            || std::env::var_os("WAVECRATE_CONFIG_PROFILE").is_some()
    }

    #[test]
    fn log_filename_has_timestamp_run_discriminator_and_sequence() {
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let timestamp = format_log_timestamp(fixed).unwrap();
        let name = format_segment_file_name(&timestamp, 2, 11);
        assert_eq!(
            name,
            "wavecrate_2023-11-14_22-13-20_0000000000000002_000000000000000b.log"
        );
        assert_eq!(parse_run_ordinal(&name, &timestamp), Some(2));
    }

    #[test]
    fn rapid_rotations_and_same_second_relaunch_have_unique_ordered_names() {
        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (mut first_run, first) = start_log_run(dir.path(), fixed).unwrap();
        let mut ordered = vec![first];
        for _ in 0..12 {
            let (next, file) = first_run.open_next().unwrap();
            drop(file);
            assert!(ordered.last().unwrap() < &next);
            ordered.push(next);
        }
        let (_restarted_run, restarted) = start_log_run(dir.path(), fixed).unwrap();

        assert!(ordered.last().unwrap() < &restarted);
        ordered.push(restarted.clone());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 14);
        for path in &ordered {
            assert!(path.is_file());
            set_file_mtime(path, FileTime::from_unix_time(1_700_000_000, 0)).unwrap();
        }
        assert_eq!(newest_log_file(dir.path()).unwrap(), Some(restarted));
    }

    #[test]
    fn next_segment_collision_does_not_overwrite_or_advance_sequence() {
        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (mut run, _) = start_log_run(dir.path(), fixed).unwrap();
        let collision = dir.path().join(format_segment_file_name(
            &run.timestamp,
            run.run_ordinal,
            run.next_sequence,
        ));
        fs::write(&collision, b"sentinel").unwrap();

        assert_eq!(
            run.open_next().unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&collision).unwrap(), b"sentinel");
        assert_eq!(run.next_sequence, 1);

        fs::remove_file(&collision).unwrap();
        let (created, file) = run.open_next().unwrap();
        drop(file);
        assert_eq!(created, collision);
        assert_eq!(run.next_sequence, 2);
    }

    #[test]
    fn unrelated_log_is_not_selected_or_pruned() {
        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let unrelated = dir.path().join("other.log");
        fs::write(&unrelated, b"keep").unwrap();
        let (_run, newest) = start_log_run(dir.path(), fixed).unwrap();

        prune_old_logs(dir.path(), 0, None).unwrap();
        assert!(unrelated.exists());
        assert!(!newest.exists());
        assert_eq!(newest_log_file(dir.path()).unwrap(), None);
    }

    #[test]
    fn startup_pruning_keeps_new_segment_when_older_logs_have_future_mtimes() {
        let dir = tempdir().unwrap();
        for index in 0..10 {
            let path = dir.path().join(format!("wavecrate_old_{index}.log"));
            fs::write(&path, b"old").unwrap();
            set_file_mtime(&path, FileTime::from_unix_time(2_000_000_000, 0)).unwrap();
        }
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (_run, current) = start_log_run(dir.path(), fixed).unwrap();

        prune_old_logs(dir.path(), 10, Some(&current)).unwrap();

        assert!(current.exists());
        assert_eq!(count_logs(dir.path()), 10);
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

        prune_old_logs(dir.path(), 10, None).unwrap();

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

        prune_old_logs(dir.path(), 10, None).unwrap();

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

        prune_old_logs(dir.path(), 10, None).unwrap();

        assert!(non_log_path.exists());
    }

    #[test]
    fn newest_log_file_returns_most_recent_log_only() {
        let dir = tempdir().unwrap();
        let older = dir.path().join("wavecrate_older.log");
        ensure_file_exists(&older).unwrap();
        thread::sleep(Duration::from_millis(10));
        let newer = dir.path().join("wavecrate_newer.log");
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
