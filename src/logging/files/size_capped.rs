//! Complete-event size boundary for the logging worker.

use std::{
    fs::File,
    io::{self, Seek, SeekFrom, Write},
};

/// The prospective maximum for one log segment. A single indivisible event may exceed it.
pub(super) const MAX_LOG_SEGMENT_BYTES: u64 = 10 * 1024 * 1024;

/// Writes one complete formatted event per call and rotates before crossing the segment cap.
///
/// The tracing formatter sends each event as one `write_all` call to the nonblocking worker.
/// The worker then calls `write_all` on this writer. A single event larger than the cap is
/// retained whole in an otherwise empty segment; its maximum overshoot is that event's length
/// minus the cap. A later event starts a new segment.
pub(super) struct SizeCappedLogAppender {
    current: File,
    current_len: u64,
    segment_limit: u64,
    open_next: Box<dyn FnMut() -> io::Result<File> + Send>,
}

impl SizeCappedLogAppender {
    /// Build the worker's file writer from an already opened initial segment.
    pub(super) fn new(
        current: File,
        open_next: impl FnMut() -> io::Result<File> + Send + 'static,
    ) -> io::Result<Self> {
        Self::with_limit(current, MAX_LOG_SEGMENT_BYTES, open_next)
    }

    fn with_limit(
        mut current: File,
        segment_limit: u64,
        open_next: impl FnMut() -> io::Result<File> + Send + 'static,
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
            open_next: Box::new(open_next),
        })
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
            // A failed open leaves the existing writable segment and event untouched.
            let next = (self.open_next)()?;
            if next.metadata()?.len() != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "next log segment is not empty",
                ));
            }
            self.current_len = 0;
            self.current = next;
        }

        match self.current.write_all(event) {
            Ok(()) => {
                self.current_len += event_len;
                Ok(event.len())
            }
            Err(error) => {
                // Preserve the original write error even if a later size read also fails.
                // A partial OS write may have advanced the file before returning the error.
                if let Ok(metadata) = self.current.metadata() {
                    self.current_len = metadata.len();
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
        path::Path,
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
        assert_eq!(
            writer.write_all(b"9").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678"
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
        assert_eq!(
            writer.write_all(b"9").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678"
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
        assert_eq!(
            writer.write_all(b"9").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            fs::read(dir.path().join("segment_0.log")).unwrap(),
            b"12345678"
        );
        assert_eq!(
            fs::read(dir.path().join("segment_1.log")).unwrap(),
            b"existing"
        );
    }
}
