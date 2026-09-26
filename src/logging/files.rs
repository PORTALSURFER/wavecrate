//! Log file path resolution and maintenance helpers.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

use super::LoggingError;
use crate::app_dirs;

/// Maximum number of matching regular log files to retain after successful cleanup.
const MAX_LOG_FILES: usize = 10;
const LOG_FILE_PREFIX: &str = "wavecrate";

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
#[derive(Debug)]
pub(super) struct LaunchLogFile {
    /// Absolute path to the file for startup diagnostics.
    pub(super) path: PathBuf,
    /// Keep the create-new handle open through subscriber setup.
    pub(super) file: File,
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

    /// Prune after the newest created segment has become the active writer handle.
    ///
    /// The current segment is derived from this run rather than supplied by a caller. The
    /// runtime integration must call this only after switching its writer to the new file.
    pub(super) fn prune_after_activation(&self) -> Result<(), LoggingError> {
        let active_sequence = self.next_sequence - 1;
        let active = self.dir.join(format_segment_file_name(
            &self.timestamp,
            self.run_ordinal,
            active_sequence,
        ));
        prune_old_logs(&self.dir, MAX_LOG_FILES, Some(&active))
    }
}

impl size_capped::SegmentLifecycle for LogSegmentRun {
    fn open_next(&mut self) -> io::Result<File> {
        LogSegmentRun::open_next(self).map(|(_, file)| file)
    }

    fn prune_after_activation(&mut self) -> Result<(), String> {
        LogSegmentRun::prune_after_activation(self).map_err(|error| error.to_string())
    }
}

/// Resolve the current profile/app-root/log-dir paths without installing logging.
pub(crate) fn resolve_log_profile_paths() -> Result<LogProfilePaths, LoggingError> {
    let app_root = app_dirs::app_root_dir().map_err(map_app_dir_error)?;
    let logs_dir = app_dirs::logs_dir().map_err(map_app_dir_error)?;
    Ok(LogProfilePaths { app_root, logs_dir })
}

/// Prepare the per-launch log file and prune matching logs to ten files.
/// The worker rotates at a prospective 10 MiB boundary after startup.
pub(super) fn prepare_launch_log_file() -> Result<LaunchLogFile, LoggingError> {
    let log_dir = resolve_log_profile_paths()?.logs_dir;
    let (run, log_path, file) = start_log_run(&log_dir, now_local_or_utc())?;
    prune_startup_or_report(&run, report_degraded_logging);
    Ok(LaunchLogFile {
        path: log_path,
        file,
        run,
    })
}

fn prune_startup_or_report(run: &LogSegmentRun, mut report: impl FnMut(&str, &str)) {
    if let Err(error) = run.prune_after_activation() {
        report(
            "startup retention cleanup failed; continuing",
            &error.to_string(),
        );
    }
}

/// Write one best-effort diagnostic without sending it back through the tracing worker.
pub(super) fn report_degraded_logging(stage: &str, detail: &str) {
    let first_line = detail.lines().next().unwrap_or("");
    let _ = writeln!(
        io::stderr().lock(),
        "wavecrate logging: {stage}: {first_line}"
    );
}

/// Return the newest matching regular `.log` file under one log directory.
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
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|source| LoggingError::ReadDir {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| LoggingError::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(LOG_FILE_PREFIX) || !name.ends_with(".log") {
            continue;
        }
        if !entry
            .file_type()
            .map_err(|source| LoggingError::ReadDir {
                path: dir.to_path_buf(),
                source,
            })?
            .is_file()
        {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push((modified, entry.path()));
    }
    Ok(entries)
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
) -> Result<(LogSegmentRun, PathBuf, File), LoggingError> {
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
                return Ok((
                    LogSegmentRun {
                        dir: dir.to_path_buf(),
                        timestamp,
                        run_ordinal,
                        next_sequence: 1,
                    },
                    path,
                    file,
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
    use std::{io::Write, path::Path, thread, time::Duration};
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
        let (mut first_run, first, _first_file) = start_log_run(dir.path(), fixed).unwrap();
        let mut ordered = vec![first];
        for _ in 0..12 {
            let (next, file) = first_run.open_next().unwrap();
            drop(file);
            assert!(ordered.last().unwrap() < &next);
            ordered.push(next);
        }
        let (_restarted_run, restarted, _restarted_file) =
            start_log_run(dir.path(), fixed).unwrap();

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
        let (mut run, _, _initial_file) = start_log_run(dir.path(), fixed).unwrap();
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
        let (_run, newest, initial_file) = start_log_run(dir.path(), fixed).unwrap();
        drop(initial_file);

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
        let (run, current, _initial_file) = start_log_run(dir.path(), fixed).unwrap();

        run.prune_after_activation().unwrap();

        assert!(current.exists());
        assert_eq!(count_logs(dir.path()), 10);
    }

    #[cfg(unix)]
    #[test]
    fn startup_cleanup_permission_failure_reports_and_keeps_current_writable() {
        use std::os::unix::fs::PermissionsExt;

        struct RestorePermissions(PathBuf, fs::Permissions);
        impl Drop for RestorePermissions {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, self.1.clone());
            }
        }

        let dir = tempdir().unwrap();
        for index in 0..11 {
            fs::write(
                dir.path().join(format!("wavecrate_old_{index}.log")),
                b"old",
            )
            .unwrap();
        }
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (run, current, mut current_file) = start_log_run(dir.path(), fixed).unwrap();
        let original = fs::metadata(dir.path()).unwrap().permissions();
        let restore = RestorePermissions(dir.path().to_path_buf(), original);
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();

        let mut reports = Vec::new();
        prune_startup_or_report(&run, |stage, detail| {
            reports.push(format!("{stage}: {detail}"));
        });
        current_file.write_all(b"still logging").unwrap();
        drop(restore);

        assert_eq!(reports.len(), 1);
        assert!(reports[0].contains("startup retention cleanup failed"));
        assert_eq!(fs::read(current).unwrap(), b"still logging");
        assert_eq!(log_files_by_modified_time(dir.path()).unwrap().len(), 12);
    }

    #[test]
    fn rotation_prunes_oldest_regular_logs_and_keeps_active_handle() {
        let dir = tempdir().unwrap();
        let other_profile = tempdir().unwrap();
        let legacy = dir.path().join("wavecrate_legacy.log");
        let legacy_file = File::create(&legacy).unwrap();
        legacy_file.set_len(20 * 1024 * 1024).unwrap();
        drop(legacy_file);
        set_file_mtime(&legacy, FileTime::from_unix_time(1_600_000_000, 0)).unwrap();
        #[cfg(unix)]
        let symlink_path = dir.path().join("wavecrate_link.log");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&legacy, &symlink_path).unwrap();
        let unrelated_log = dir.path().join("other.log");
        let unrelated_extension = dir.path().join("wavecrate_notes.txt");
        let unrelated_directory = dir.path().join("wavecrate_directory.log");
        let other_profile_log = other_profile.path().join("wavecrate_other.log");
        fs::write(&unrelated_log, b"keep").unwrap();
        fs::write(&unrelated_extension, b"keep").unwrap();
        fs::create_dir(&unrelated_directory).unwrap();
        fs::write(&other_profile_log, b"keep").unwrap();

        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (mut run, initial, initial_file) = start_log_run(dir.path(), fixed).unwrap();
        drop(initial_file);
        set_file_mtime(&initial, FileTime::from_unix_time(1_700_000_000, 0)).unwrap();
        let mut created = vec![initial];
        run.prune_after_activation().unwrap();
        for _ in 0..15 {
            let (path, mut active_file) = run.open_next().unwrap();
            active_file.write_all(b"event").unwrap();
            set_file_mtime(&path, FileTime::from_unix_time(1_700_000_000, 0)).unwrap();
            run.prune_after_activation().unwrap();
            assert!(path.is_file());
            assert!(log_files_by_modified_time(dir.path()).unwrap().len() <= 10);
            active_file.write_all(b" retained").unwrap();
            set_file_mtime(&path, FileTime::from_unix_time(1_700_000_000, 0)).unwrap();
            created.push(path);
        }

        assert_eq!(log_files_by_modified_time(dir.path()).unwrap().len(), 10);
        assert!(!legacy.exists());
        for obsolete in &created[..created.len() - 10] {
            assert!(!obsolete.exists());
        }
        for retained in &created[created.len() - 10..] {
            assert!(retained.is_file());
        }
        assert_eq!(
            newest_log_file(dir.path()).unwrap(),
            created.last().cloned()
        );
        assert_eq!(fs::read(&unrelated_log).unwrap(), b"keep");
        assert_eq!(fs::read(&unrelated_extension).unwrap(), b"keep");
        assert!(unrelated_directory.is_dir());
        assert_eq!(fs::read(&other_profile_log).unwrap(), b"keep");
        #[cfg(unix)]
        assert!(
            fs::symlink_metadata(&symlink_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
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
