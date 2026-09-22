//! Complete-event size boundary for the logging worker.

use std::{
    fs::File,
    io::{self, Seek, SeekFrom, Write},
};

/// The prospective maximum for one log segment. A single indivisible event may exceed it.
pub(super) const MAX_LOG_SEGMENT_BYTES: u64 = 10 * 1024 * 1024;

/// Worker-local creation and retention steps for a log run.
pub(in crate::logging) trait SegmentLifecycle: Send {
    fn open_next(&mut self) -> io::Result<File>;

    fn prune_after_activation(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl<F> SegmentLifecycle for F
where
    F: FnMut() -> io::Result<File> + Send,
{
    fn open_next(&mut self) -> io::Result<File> {
        self()
    }
}

/// Writes one complete formatted event per call and rotates before crossing the segment cap.
///
/// The tracing formatter sends each event as one `write_all` call to the nonblocking worker.
/// The worker then calls `write_all` on this writer. A single event larger than the cap is
/// retained whole in an otherwise empty segment; with successful rotation, its maximum overshoot
/// is that event's length minus the cap. If rotation fails, the current file remains writable and
/// may exceed the cap; a diagnostic makes that degraded retention explicit.
pub(in crate::logging) struct SizeCappedLogAppender {
    current: File,
    current_len: u64,
    segment_limit: u64,
    lifecycle: Box<dyn SegmentLifecycle>,
    report: Box<dyn FnMut(&'static str, &str) + Send>,
    rotation_error_reported: bool,
    cleanup_error_reported: bool,
    write_error_reported: bool,
}

impl SizeCappedLogAppender {
    /// Build the worker's file writer from an already opened initial segment.
    pub(in crate::logging) fn new(
        current: File,
        lifecycle: impl SegmentLifecycle + 'static,
    ) -> io::Result<Self> {
        Self::with_limit(current, MAX_LOG_SEGMENT_BYTES, lifecycle)
    }

    fn with_limit(
        current: File,
        segment_limit: u64,
        lifecycle: impl SegmentLifecycle + 'static,
    ) -> io::Result<Self> {
        Self::with_limit_and_reporter(current, segment_limit, lifecycle, |stage, detail| {
            super::report_degraded_logging(stage, detail);
        })
    }

    fn with_limit_and_reporter(
        mut current: File,
        segment_limit: u64,
        lifecycle: impl SegmentLifecycle + 'static,
        report: impl FnMut(&'static str, &str) + Send + 'static,
    ) -> io::Result<Self> {
        if segment_limit == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log segment limit must be positive",
            ));
        }
        let current_len = current.seek(SeekFrom::End(0))?;
        Ok(Self {
            current,
            current_len,
            segment_limit,
            lifecycle: Box::new(lifecycle),
            report: Box::new(report),
            rotation_error_reported: false,
            cleanup_error_reported: false,
            write_error_reported: false,
        })
    }

    fn report_rotation_failure(&mut self, error: &io::Error) {
        if !self.rotation_error_reported {
            (self.report)(
                "rotation failed; continuing in current segment without size bound",
                &error.to_string(),
            );
            self.rotation_error_reported = true;
        }
    }

    fn rotate_or_keep_current(&mut self) {
        let next = match self.lifecycle.open_next() {
            Ok(next) => next,
            Err(error) => {
                self.report_rotation_failure(&error);
                return;
            }
        };
        match next.metadata() {
            Ok(metadata) if metadata.len() == 0 => {}
            Ok(_) => {
                self.report_rotation_failure(&io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "next log segment is not empty",
                ));
                return;
            }
            Err(error) => {
                self.report_rotation_failure(&error);
                return;
            }
        }

        self.current = next;
        self.current_len = 0;
        self.rotation_error_reported = false;
        match self.lifecycle.prune_after_activation() {
            Ok(()) => self.cleanup_error_reported = false,
            Err(error) if !self.cleanup_error_reported => {
                (self.report)(
                    "retention cleanup failed; continuing in active segment",
                    &error,
                );
                self.cleanup_error_reported = true;
            }
            Err(_) => {}
        }
    }
}

impl Write for SizeCappedLogAppender {
    fn write(&mut self, event: &[u8]) -> io::Result<usize> {
        if event.is_empty() {
            return Ok(0);
        }

        let event_len = u64::try_from(event.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "log event length exceeds u64")
        })?;
        if self.current_len != 0
            && self
                .current_len
                .checked_add(event_len)
                .is_none_or(|total| total > self.segment_limit)
        {
            self.rotate_or_keep_current();
        }

        match self.current.write_all(event) {
            Ok(()) => {
                self.current_len = self.current_len.saturating_add(event_len);
                self.write_error_reported = false;
                Ok(event.len())
            }
            Err(error) => {
                // Preserve the original write error even if a later size read also fails.
                // A partial OS write may have advanced the file before returning the error.
                if let Ok(metadata) = self.current.metadata() {
                    self.current_len = metadata.len();
                } else {
                    self.current_len = u64::MAX;
                }
                if !self.write_error_reported {
                    (self.report)("active log write failed", &error.to_string());
                    self.write_error_reported = true;
                }
                Err(error)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.current.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, OpenOptions},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;

    fn segment_writer(dir: &Path, limit: u64) -> SizeCappedLogAppender {
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join("segment_0.log"))
            .unwrap();
        let dir = dir.to_path_buf();
        let mut sequence = 0usize;
        SizeCappedLogAppender::with_limit(initial, limit, move || {
            sequence += 1;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(format!("segment_{sequence}.log")))
        })
        .unwrap()
    }

    #[test]
    fn segment_cap_is_ten_mib_and_writer_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<SizeCappedLogAppender>();
        let dir = tempdir().unwrap();
        let current = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let writer = SizeCappedLogAppender::new(current, || unreachable!()).unwrap();
        assert_eq!(writer.segment_limit, 10 * 1024 * 1024);
    }

    #[test]
    fn complete_event_at_exact_boundary_stays_in_current_segment() {
        let dir = tempdir().unwrap();
        let mut writer = segment_writer(dir.path(), 8);

        writer.write_all(b"12345678").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678"
        );
        assert!(!dir.path().join("segment_1.log").exists());

        writer.write_all(b"9").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678"
        );
        assert_eq!(fs::read(dir.path().join("segment_1.log")).unwrap(), b"9");
    }

    #[test]
    fn existing_initial_segment_appends_at_its_end() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("segment_0.log");
        fs::write(&path, b"old").unwrap();
        let current = OpenOptions::new().write(true).open(&path).unwrap();
        let mut writer = SizeCappedLogAppender::with_limit(current, 8, || unreachable!()).unwrap();

        writer.write_all(b"new").unwrap();
        assert_eq!(fs::read(path).unwrap(), b"oldnew");
    }

    #[test]
    fn over_cap_event_rotates_before_writing_any_of_its_bytes() {
        let dir = tempdir().unwrap();
        let mut writer = segment_writer(dir.path(), 8);

        writer.write_all(b"1234567").unwrap();
        writer.write_all(b"ab").unwrap();

        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"1234567"
        );
        assert_eq!(fs::read(dir.path().join("segment_1.log")).unwrap(), b"ab");
    }

    #[test]
    fn oversized_event_is_whole_and_only_one_event_overshoots() {
        let dir = tempdir().unwrap();
        let mut writer = segment_writer(dir.path(), 8);

        writer.write_all(b"old").unwrap();
        writer.write_all(b"012345678901").unwrap();
        writer.write_all(b"new").unwrap();

        assert_eq!(fs::read(dir.path().join("segment_0.log")).unwrap(), b"old");
        assert_eq!(
            fs::read(dir.path().join("segment_1.log")).unwrap(),
            b"012345678901"
        );
        assert_eq!(fs::read(dir.path().join("segment_2.log")).unwrap(), b"new");
    }

    #[test]
    fn failed_rotation_open_keeps_current_segment_and_can_retry() {
        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let next = dir.path().join("segment_1.log");
        let mut attempts = 0;
        let mut writer = SizeCappedLogAppender::with_limit(initial, 8, move || {
            attempts += 1;
            if attempts == 1 {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
            }
            OpenOptions::new().write(true).create_new(true).open(&next)
        })
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"123456789"
        );
        assert!(!dir.path().join("segment_1.log").exists());

        writer.write_all(b"9").unwrap();
        assert_eq!(fs::read(dir.path().join("segment_1.log")).unwrap(), b"9");
    }

    #[test]
    fn missing_next_segment_directory_preserves_current_bytes() {
        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let missing = dir.path().join("missing").join("segment_1.log");
        let mut writer = SizeCappedLogAppender::with_limit(initial, 8, move || {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&missing)
        })
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"123456789"
        );
    }

    #[test]
    fn rotation_rejects_a_nonempty_next_segment() {
        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let next = dir.path().join("segment_1.log");
        fs::write(&next, b"existing").unwrap();
        let mut writer = SizeCappedLogAppender::with_limit(initial, 8, move || {
            OpenOptions::new().append(true).open(&next)
        })
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"123456789"
        );
        assert_eq!(
            fs::read(dir.path().join("segment_1.log")).unwrap(),
            b"existing"
        );
    }

    #[test]
    fn repeated_rotation_failure_reports_once_and_keeps_logging_writable() {
        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let next = dir.path().join("segment_1.log");
        let mut attempts = 0;
        let reports = Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = Arc::clone(&reports);
        let mut writer = SizeCappedLogAppender::with_limit_and_reporter(
            initial,
            8,
            move || {
                attempts += 1;
                if attempts <= 2 {
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
                }
                OpenOptions::new().write(true).create_new(true).open(&next)
            },
            move |stage, detail| captured.lock().unwrap().push(format!("{stage}: {detail}")),
        )
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"a").unwrap();
        writer.write_all(b"b").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678ab"
        );
        assert_eq!(reports.lock().unwrap().len(), 1);
        assert!(reports.lock().unwrap()[0].contains("without size bound"));

        writer.write_all(b"c").unwrap();
        assert_eq!(fs::read(dir.path().join("segment_1.log")).unwrap(), b"c");
        assert_eq!(reports.lock().unwrap().len(), 1);
    }

    struct CleanupFault {
        dir: PathBuf,
        sequence: usize,
        remaining_failures: usize,
    }

    impl SegmentLifecycle for CleanupFault {
        fn open_next(&mut self) -> io::Result<File> {
            self.sequence += 1;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.dir.join(format!("segment_{}.log", self.sequence)))
        }

        fn prune_after_activation(&mut self) -> Result<(), String> {
            if self.remaining_failures > 0 {
                self.remaining_failures -= 1;
                return Err(String::from("simulated cleanup denied"));
            }
            Ok(())
        }
    }

    #[test]
    fn cleanup_failure_reports_and_keeps_new_segment_writable() {
        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let reports = Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = Arc::clone(&reports);
        let mut writer = SizeCappedLogAppender::with_limit_and_reporter(
            initial,
            8,
            CleanupFault {
                dir: dir.path().to_path_buf(),
                sequence: 0,
                remaining_failures: 1,
            },
            move |stage, detail| captured.lock().unwrap().push(format!("{stage}: {detail}")),
        )
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(fs::read(dir.path().join("segment_1.log")).unwrap(), b"9");
        assert_eq!(reports.lock().unwrap().len(), 1);
        assert!(reports.lock().unwrap()[0].contains("retention cleanup failed"));

        writer.write_all(b"abcdefgh").unwrap();
        assert_eq!(
            fs::read(dir.path().join("segment_2.log")).unwrap(),
            b"abcdefgh"
        );
        assert_eq!(reports.lock().unwrap().len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn locked_next_segment_collision_keeps_current_file_writable() {
        use std::os::fd::AsRawFd;
        use time::OffsetDateTime;

        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (run, initial_path, current) = super::super::start_log_run(dir.path(), fixed).unwrap();
        let next_path = dir.path().join(super::super::format_segment_file_name(
            &run.timestamp,
            run.run_ordinal,
            run.next_sequence,
        ));
        let locked = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&next_path)
            .unwrap();
        assert_eq!(
            unsafe { libc::flock(locked.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );

        let reports = Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = Arc::clone(&reports);
        let mut writer = SizeCappedLogAppender::with_limit_and_reporter(
            current,
            8,
            run,
            move |stage, detail| captured.lock().unwrap().push(format!("{stage}: {detail}")),
        )
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(fs::read(&initial_path).unwrap(), b"123456789");
        assert_eq!(reports.lock().unwrap().len(), 1);
        assert!(fs::read(&next_path).unwrap().is_empty());

        drop(locked);
        fs::remove_file(&next_path).unwrap();
        writer.write_all(b"x").unwrap();
        assert_eq!(fs::read(next_path).unwrap(), b"x");
    }

    #[test]
    fn nonblocking_worker_rotates_real_run_without_reopening_initial_segment() {
        use time::OffsetDateTime;

        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (run, initial_path, initial_file) =
            super::super::start_log_run(dir.path(), fixed).unwrap();
        let next_path = dir.path().join(super::super::format_segment_file_name(
            &run.timestamp,
            run.run_ordinal,
            run.next_sequence,
        ));
        let appender = SizeCappedLogAppender::with_limit(initial_file, 8, run).unwrap();
        let (mut worker, guard) = tracing_appender::non_blocking(appender);

        worker.write_all(b"12345678").unwrap();
        worker.write_all(b"9").unwrap();
        drop(worker);
        drop(guard);

        assert_eq!(fs::read(initial_path).unwrap(), b"12345678");
        assert_eq!(fs::read(next_path).unwrap(), b"9");
    }

    #[test]
    fn temporary_profile_stress_retains_ten_ordered_segments_after_many_rotations() {
        use time::OffsetDateTime;

        let base = tempdir().unwrap();
        let logs = base.path().join(".wavecrate/profiles/automated-tests/logs");
        fs::create_dir_all(&logs).unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (run, initial_path, initial_file) = super::super::start_log_run(&logs, fixed).unwrap();
        let names = (0..=17)
            .map(|sequence| {
                logs.join(super::super::format_segment_file_name(
                    &run.timestamp,
                    run.run_ordinal,
                    sequence,
                ))
            })
            .collect::<Vec<_>>();
        assert_eq!(initial_path, names[0]);
        let appender = SizeCappedLogAppender::with_limit(initial_file, 64, run).unwrap();
        let (mut worker, guard) = tracing_appender::non_blocking(appender);

        for index in 0..16 {
            worker
                .write_all(format!("{index:02}").repeat(32).as_bytes())
                .unwrap();
        }
        worker.write_all(&[b'X'; 96]).unwrap();
        worker.write_all(b"final").unwrap();
        drop(worker);
        drop(guard);

        let mut retained = super::super::log_files_by_modified_time(&logs)
            .unwrap()
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();
        retained.sort();
        assert_eq!(retained, names[8..=17]);
        assert!(names[..8].iter().all(|path| !path.exists()));
        assert!(retained
            .iter()
            .all(|path| fs::metadata(path).unwrap().len() <= 96));
        assert_eq!(fs::read(&names[16]).unwrap(), [b'X'; 96]);
        assert_eq!(fs::read(&names[17]).unwrap(), b"final");
        assert_eq!(
            super::super::newest_log_file(&logs).unwrap(),
            Some(names[17].clone())
        );
    }

    #[cfg(unix)]
    #[test]
    fn renamed_initial_segment_remains_writable_through_open_handle() {
        use time::OffsetDateTime;

        let dir = tempdir().unwrap();
        let fixed = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let (run, original, current) = super::super::start_log_run(dir.path(), fixed).unwrap();
        let renamed = dir.path().join("wavecrate_renamed.log");
        fs::rename(&original, &renamed).unwrap();

        let mut writer = SizeCappedLogAppender::new(current, run).unwrap();
        writer.write_all(b"still logging").unwrap();

        assert!(!original.exists());
        assert_eq!(fs::read(renamed).unwrap(), b"still logging");
    }

    #[cfg(unix)]
    #[test]
    fn read_only_log_directory_falls_back_to_open_current_file() {
        use std::os::unix::fs::PermissionsExt;

        struct RestorePermissions(PathBuf, fs::Permissions);
        impl Drop for RestorePermissions {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, self.1.clone());
            }
        }

        let dir = tempdir().unwrap();
        let initial = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.path().join("segment_0.log"))
            .unwrap();
        let next = dir.path().join("segment_1.log");
        let next_for_open = next.clone();
        let original = fs::metadata(dir.path()).unwrap().permissions();
        let restore = RestorePermissions(dir.path().to_path_buf(), original);
        let reports = Arc::new(Mutex::new(Vec::<String>::new()));
        let captured = Arc::clone(&reports);
        let mut writer = SizeCappedLogAppender::with_limit_and_reporter(
            initial,
            8,
            move || {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&next_for_open)
            },
            move |stage, detail| captured.lock().unwrap().push(format!("{stage}: {detail}")),
        )
        .unwrap();

        writer.write_all(b"12345678").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(reports.lock().unwrap().len(), 1);
        drop(restore);

        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"123456789"
        );
        writer.write_all(b"x").unwrap();
        assert_eq!(fs::read(next).unwrap(), b"x");
    }
}
