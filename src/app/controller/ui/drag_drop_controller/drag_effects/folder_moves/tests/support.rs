use super::super::super::super::file_metadata;
use crate::app::controller::jobs::FolderMoveRequest;
use crate::app::controller::test_support::write_test_wav;
use crate::sample_sources::db::DB_FILE_NAME;
use crate::sample_sources::{Rating, SampleSource, SourceDatabase};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::tempdir;

static FOLDER_MOVE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Serialize folder-move worker tests that share global test hooks.
pub(super) fn folder_move_test_guard() -> MutexGuard<'static, ()> {
    FOLDER_MOVE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("folder-move test lock poisoned")
}

/// Convenience assertion helper to avoid `unwrap`/`expect` in tests.
pub(super) trait Must<T> {
    /// Return the wrapped value or panic with a deterministic failure message.
    fn must(self) -> T;
}

impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
    fn must(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("unexpected error: {err}"),
        }
    }
}

impl<T> Must<T> for Option<T> {
    fn must(self) -> T {
        match self {
            Some(value) => value,
            None => panic!("expected value, found none"),
        }
    }
}

/// Read test fixture file metadata through the drag-drop controller helper.
pub(super) fn read_file_metadata(path: &Path) -> (u64, i64) {
    file_metadata(path).must()
}

/// Build a minimal in-source folder-move fixture with one tracked sample.
pub(super) fn setup_folder_move_fixture() -> (tempfile::TempDir, SampleSource, PathBuf) {
    let temp = tempdir().must();
    let source_root = temp.path().join("source");
    let old_dir = source_root.join("old");
    let target_dir = source_root.join("dest");
    std::fs::create_dir_all(&old_dir).must();
    std::fs::create_dir_all(&target_dir).must();
    let source = SampleSource::new(source_root.clone());
    let wav_path = old_dir.join("one.wav");
    write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
    let (file_size, modified_ns) = read_file_metadata(&wav_path);
    let db = SourceDatabase::open(&source_root).must();
    let mut batch = db.write_batch().must();
    batch
        .upsert_file(Path::new("old/one.wav"), file_size, modified_ns)
        .must();
    batch
        .set_tag(Path::new("old/one.wav"), Rating::KEEP_1)
        .must();
    batch.set_looped(Path::new("old/one.wav"), true).must();
    batch.set_locked(Path::new("old/one.wav"), true).must();
    batch
        .set_last_played_at(Path::new("old/one.wav"), 42)
        .must();
    batch.commit().must();
    (temp, source, source_root)
}

/// Construct a folder-move request relative to the standard folder-move fixture.
pub(super) fn folder_move_request(
    source: &SampleSource,
    source_root: &Path,
    folder: &str,
    target_folder: &str,
) -> FolderMoveRequest {
    FolderMoveRequest {
        source_id: source.id.clone(),
        source_root: source_root.to_path_buf(),
        folder: PathBuf::from(folder),
        target_folder: PathBuf::from(target_folder),
    }
}

/// Hold an immediate SQLite transaction open until the returned sender is released.
pub(super) fn lock_db_until_released(
    source_root: &Path,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).must();
        conn.execute_batch("BEGIN IMMEDIATE").must();
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().must();
    (lock_release_tx, lock_done_rx)
}
