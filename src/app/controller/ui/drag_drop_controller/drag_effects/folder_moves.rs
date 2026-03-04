/// UI-state/result application routines after move workers complete.
mod apply_result;
/// Drag/drop planning and validation entrypoints for folder and sample moves.
mod plan;
/// Background worker tasks that execute filesystem/database move operations.
mod worker;

#[cfg(test)]
mod tests {
    use super::super::super::file_metadata;
    use super::worker::{
        run_folder_move_task, run_folder_sample_move_task, set_before_folder_sample_batch_hook,
    };
    use crate::app::controller::jobs::{FolderMoveRequest, FolderSampleMoveRequest};
    use crate::app::controller::test_support::write_test_wav;
    use crate::sample_sources::db::{DB_FILE_NAME, file_ops_journal};
    use crate::sample_sources::{Rating, SampleSource, SourceDatabase};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, atomic::AtomicBool};
    use std::time::Duration;
    use tempfile::tempdir;

    /// Convenience assertion helper to avoid `unwrap`/`expect` in tests.
    trait Must<T> {
        /// Return the wrapped value or panic with a deterministic failure message.
        fn must(self) -> T;
    }

    impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
        /// Return the `Ok` value or panic with the `Display` form of the error.
        fn must(self) -> T {
            match self {
                Ok(value) => value,
                Err(err) => panic!("unexpected error: {err}"),
            }
        }
    }

    impl<T> Must<T> for Option<T> {
        /// Return the contained value or panic when the option is empty.
        fn must(self) -> T {
            match self {
                Some(value) => value,
                None => panic!("expected value, found none"),
            }
        }
    }

    #[test]
    /// Moving a single file updates filesystem state, DB metadata, and clears journal entries.
    fn folder_sample_move_updates_db_entry() {
        let temp = tempdir().must();
        let source_root = temp.path().join("source");
        let target_dir = source_root.join("folder");
        std::fs::create_dir_all(&target_dir).must();
        let source = SampleSource::new(source_root.clone());
        let wav_path = source_root.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).must();
        let db = SourceDatabase::open(&source_root).must();
        let mut batch = db.write_batch().must();
        batch
            .upsert_file(Path::new("one.wav"), file_size, modified_ns)
            .must();
        batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).must();
        batch.set_looped(Path::new("one.wav"), true).must();
        batch.set_last_played_at(Path::new("one.wav"), 42).must();
        batch.commit().must();

        let request = FolderSampleMoveRequest {
            relative_path: PathBuf::from("one.wav"),
            target_relative: PathBuf::from("folder/one.wav"),
        };
        let result = run_folder_sample_move_task(
            source.id.clone(),
            source_root.clone(),
            vec![request],
            Vec::new(),
            Arc::new(AtomicBool::new(false)),
            None,
        );

        assert!(result.errors.is_empty());
        assert_eq!(result.moved.len(), 1);
        assert!(source_root.join("folder/one.wav").is_file());

        let db = SourceDatabase::open(&source_root).must();
        assert!(db.tag_for_path(Path::new("one.wav")).must().is_none());
        assert_eq!(
            db.tag_for_path(Path::new("folder/one.wav")).must(),
            Some(Rating::KEEP_1)
        );
        assert_eq!(
            db.looped_for_path(Path::new("folder/one.wav")).must(),
            Some(true)
        );
        assert_eq!(
            db.last_played_at_for_path(Path::new("folder/one.wav"))
                .must(),
            Some(42)
        );
        let entries = file_ops_journal::list_entries(&db).must();
        assert!(entries.entries.is_empty());
        assert!(entries.malformed.is_empty());
    }

    #[test]
    /// Cancellation before the batch starts leaves source file and DB entries unchanged.
    fn folder_sample_move_cancelled_before_processing_keeps_source_unchanged() {
        let temp = tempdir().must();
        let source_root = temp.path().join("source");
        let target_dir = source_root.join("folder");
        std::fs::create_dir_all(&target_dir).must();
        let source = SampleSource::new(source_root.clone());
        let wav_path = source_root.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).must();
        let db = SourceDatabase::open(&source_root).must();
        db.upsert_file(Path::new("one.wav"), file_size, modified_ns)
            .must();
        db.set_tag(Path::new("one.wav"), Rating::KEEP_1).must();

        let cancel = Arc::new(AtomicBool::new(true));
        let result = run_folder_sample_move_task(
            source.id.clone(),
            source_root.clone(),
            vec![FolderSampleMoveRequest {
                relative_path: PathBuf::from("one.wav"),
                target_relative: PathBuf::from("folder/one.wav"),
            }],
            Vec::new(),
            cancel,
            None,
        );

        assert!(result.cancelled);
        assert!(result.moved.is_empty());
        assert!(result.errors.is_empty());
        assert!(source_root.join("one.wav").is_file());
        assert!(!source_root.join("folder/one.wav").exists());
        let db = SourceDatabase::open(&source_root).must();
        assert_eq!(
            db.tag_for_path(Path::new("one.wav")).must(),
            Some(Rating::KEEP_1)
        );
        let entries = file_ops_journal::list_entries(&db).must();
        assert!(entries.entries.is_empty());
        assert!(entries.malformed.is_empty());
    }

    #[test]
    /// A DB-write failure rolls the file back to source and keeps staged journal data for recovery.
    fn folder_sample_move_db_write_failure_rolls_back_source_and_keeps_journal_for_recovery() {
        let temp = tempdir().must();
        let source_root = temp.path().join("source");
        let target_dir = source_root.join("folder");
        std::fs::create_dir_all(&target_dir).must();
        let source = SampleSource::new(source_root.clone());
        let wav_path = source_root.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).must();
        let db = SourceDatabase::open(&source_root).must();
        db.upsert_file(Path::new("one.wav"), file_size, modified_ns)
            .must();
        db.set_tag(Path::new("one.wav"), Rating::KEEP_1).must();
        db.set_looped(Path::new("one.wav"), true).must();
        db.set_last_played_at(Path::new("one.wav"), 42).must();

        let source_root_for_hook = source_root.clone();
        set_before_folder_sample_batch_hook(Some(Box::new(move || {
            let (locked_tx, locked_rx) = std::sync::mpsc::channel();
            let db_file = source_root_for_hook.join(DB_FILE_NAME);
            std::thread::spawn(move || {
                let conn = rusqlite::Connection::open(db_file).must();
                conn.execute_batch("BEGIN IMMEDIATE").must();
                let _ = locked_tx.send(());
                std::thread::sleep(Duration::from_millis(7_000));
                let _ = conn.execute_batch("COMMIT");
            });
            locked_rx.recv_timeout(Duration::from_secs(1)).must();
        })));

        let result = run_folder_sample_move_task(
            source.id.clone(),
            source_root.clone(),
            vec![FolderSampleMoveRequest {
                relative_path: PathBuf::from("one.wav"),
                target_relative: PathBuf::from("folder/one.wav"),
            }],
            Vec::new(),
            Arc::new(AtomicBool::new(false)),
            None,
        );
        set_before_folder_sample_batch_hook(None);

        assert!(result.moved.is_empty());
        assert!(result.errors.iter().any(|err| {
            err.contains("Failed to start database update")
                || err.contains("Failed to drop old entry")
                || err.contains("Failed to register moved file")
                || err.contains("Failed to copy tag")
                || err.contains("Failed to copy loop marker")
                || err.contains("Failed to copy playback age")
                || err.contains("Failed to save move")
        }));
        assert!(source_root.join("one.wav").is_file());
        assert!(!source_root.join("folder/one.wav").exists());
        let db = SourceDatabase::open(&source_root).must();
        assert_eq!(
            db.tag_for_path(Path::new("one.wav")).must(),
            Some(Rating::KEEP_1)
        );
        let entries = file_ops_journal::list_entries(&db).must();
        assert!(entries.malformed.is_empty());
        assert_eq!(entries.entries.len(), 1);
        assert_eq!(
            entries.entries[0].stage,
            file_ops_journal::FileOpStage::Staged
        );
        assert_eq!(
            entries.entries[0].target_relative,
            PathBuf::from("folder/one.wav")
        );
    }

    #[test]
    /// Moving a folder relocates contained files and rewrites their source DB paths.
    fn folder_move_updates_db_entries() {
        let temp = tempdir().must();
        let source_root = temp.path().join("source");
        let old_dir = source_root.join("old");
        let target_dir = source_root.join("dest");
        std::fs::create_dir_all(&old_dir).must();
        std::fs::create_dir_all(&target_dir).must();
        let source = SampleSource::new(source_root.clone());
        let wav_path = old_dir.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).must();
        let db = SourceDatabase::open(&source_root).must();
        let mut batch = db.write_batch().must();
        batch
            .upsert_file(Path::new("old/one.wav"), file_size, modified_ns)
            .must();
        batch
            .set_tag(Path::new("old/one.wav"), Rating::KEEP_1)
            .must();
        batch.commit().must();

        let request = FolderMoveRequest {
            source_id: source.id.clone(),
            source_root: source_root.clone(),
            folder: PathBuf::from("old"),
            target_folder: PathBuf::from("dest"),
        };
        let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);

        assert!(result.errors.is_empty());
        assert_eq!(result.moved.len(), 1);
        assert!(source_root.join("dest/old/one.wav").is_file());

        let db = SourceDatabase::open(&source_root).must();
        assert!(db.tag_for_path(Path::new("old/one.wav")).must().is_none());
        assert_eq!(
            db.tag_for_path(Path::new("dest/old/one.wav")).must(),
            Some(Rating::KEEP_1)
        );
    }
}
