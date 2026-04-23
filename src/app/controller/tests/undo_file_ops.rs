use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use super::super::*;
use crate::app::controller::jobs::{FileOpResult, UndoFileJob, UndoFileOpResult, UndoFileOutcome};
use crate::app::controller::undo::{DeferredUndo, UndoDirection, UndoEntry, UndoExecution};
use crate::app::controller::undo_jobs::run_undo_file_job;
use crate::sample_sources::SourceDatabase;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::AtomicBool,
    mpsc::{Receiver, Sender},
};
use std::time::Duration;
use tempfile::tempdir;

fn deferred_test_entry(
    label: &str,
    undo_value: bool,
    redo_value: bool,
) -> UndoEntry<AppController> {
    let label = label.to_string();
    UndoEntry::new(
        label,
        move |controller: &mut AppController| {
            controller.settings.controls.advance_after_rating = undo_value;
            Ok(UndoExecution::Applied)
        },
        move |controller: &mut AppController| {
            controller.settings.controls.advance_after_rating = redo_value;
            Ok(UndoExecution::Applied)
        },
    )
}

fn deferred_remove_job(source: &SampleSource, relative_path: &str) -> UndoFileJob {
    let relative_path = PathBuf::from(relative_path);
    UndoFileJob::RemoveSample {
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        absolute_path: source.root.join(&relative_path),
        relative_path,
    }
}

#[test]
fn deferred_undo_success_updates_entry_and_pushes_redo() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.settings.controls.advance_after_rating = false;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("remove sample", false, true),
        direction: UndoDirection::Undo,
        job: UndoFileJob::Overwrite {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: PathBuf::from("one.wav"),
            absolute_path: source.root.join("one.wav"),
            backup_path: source.root.join("undo-before.wav"),
        },
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Overwrite {
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            file_size: 42,
            modified_ns: 7,
            tag: crate::sample_sources::Rating::KEEP_1,
            looped: true,
            last_played_at: Some(11),
        }),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());
    let updated_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    let updated = controller.wav_entry(updated_index).unwrap();
    assert_eq!(updated.file_size, 42);
    assert_eq!(updated.modified_ns, 7);
    assert_eq!(updated.tag, crate::sample_sources::Rating::KEEP_1);
    assert!(updated.looped);
    assert_eq!(updated.last_played_at, Some(11));

    controller.redo();
    assert!(controller.settings.controls.advance_after_rating);
}

#[test]
fn deferred_undo_cancellation_restores_undo_entry() {
    let (mut controller, source) = dummy_controller();
    controller.settings.controls.advance_after_rating = true;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("deferred undo", false, true),
        direction: UndoDirection::Undo,
        job: deferred_remove_job(&source, "one.wav"),
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Err("ignored after cancellation".to_string()),
        cancelled: true,
    }));

    assert!(controller.history.pending_undo.is_none());

    controller.undo();
    assert!(!controller.settings.controls.advance_after_rating);
}

#[test]
fn deferred_redo_failure_restores_redo_entry() {
    let (mut controller, source) = dummy_controller();
    controller.settings.controls.advance_after_rating = false;
    controller.history.pending_undo = Some(DeferredUndo {
        entry: deferred_test_entry("deferred redo", false, true),
        direction: UndoDirection::Redo,
        job: deferred_remove_job(&source, "one.wav"),
    });

    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Err("redo failed".to_string()),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());

    controller.redo();
    assert!(controller.settings.controls.advance_after_rating);
}

#[test]
fn remove_sample_job_treats_missing_file_as_idempotent_when_db_cleanup_succeeds() {
    let (temp, source, relative_path, absolute_path) = remove_sample_fixture("missing.wav");
    std::fs::remove_file(&absolute_path).expect("remove fixture file before job");

    let result = run_undo_file_job(
        UndoFileJob::RemoveSample {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: relative_path.clone(),
            absolute_path: absolute_path.clone(),
        },
        Arc::new(AtomicBool::new(false)),
        None,
    );

    assert!(
        matches!(
            &result.result,
            Ok(UndoFileOutcome::Removed {
                source_id,
                relative_path: outcome_path,
            }) if *source_id == source.id && *outcome_path == relative_path
        ),
        "missing file should be an idempotent success: {:?}",
        result.result
    );
    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert!(
        db.tag_for_path(&relative_path)
            .expect("lookup removed row")
            .is_none()
    );

    drop(temp);
}

#[test]
fn remove_sample_job_fails_when_filesystem_delete_fails() {
    let (temp, source, relative_path, absolute_path) = remove_sample_fixture("locked.wav");
    std::fs::remove_file(&absolute_path).expect("remove fixture file");
    std::fs::create_dir(&absolute_path).expect("create directory at sample path");

    let result = run_undo_file_job(
        UndoFileJob::RemoveSample {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: relative_path.clone(),
            absolute_path: absolute_path.clone(),
        },
        Arc::new(AtomicBool::new(false)),
        None,
    );

    let err = result.result.expect_err("directory delete should fail");
    assert!(err.contains("Failed to delete sample"));
    assert!(
        absolute_path.is_dir(),
        "failed delete must leave path in place"
    );
    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert_eq!(
        db.tag_for_path(&relative_path)
            .expect("db row should remain"),
        Some(crate::sample_sources::Rating::NEUTRAL)
    );

    drop(temp);
}

#[test]
fn remove_sample_job_fails_when_db_cleanup_fails() {
    let (temp, source, relative_path, absolute_path) = remove_sample_fixture("db-lock.wav");
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    let result = run_undo_file_job(
        UndoFileJob::RemoveSample {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: relative_path.clone(),
            absolute_path: absolute_path.clone(),
        },
        Arc::new(AtomicBool::new(false)),
        None,
    );

    release_db_lock(lock_release_tx, lock_done_rx);

    let err = result.result.expect_err("locked db cleanup should fail");
    assert!(err.contains("Failed to drop database row"));
    assert!(
        !absolute_path.exists(),
        "file delete happens before the db cleanup failure is surfaced"
    );
    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert_eq!(
        db.tag_for_path(&relative_path)
            .expect("db row should remain"),
        Some(crate::sample_sources::Rating::NEUTRAL)
    );

    drop(temp);
}

fn remove_sample_fixture(sample_name: &str) -> (tempfile::TempDir, SampleSource, PathBuf, PathBuf) {
    let temp = tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative_path = PathBuf::from(sample_name);
    let absolute_path = source.root.join(&relative_path);
    write_test_wav(&absolute_path, &[0.0, 0.1, -0.1]);
    let metadata = std::fs::metadata(&absolute_path).expect("read sample metadata");
    let db = SourceDatabase::open(&source.root).expect("open source db");
    db.upsert_file(&relative_path, metadata.len(), 0)
        .expect("insert db row");
    db.set_tag(&relative_path, crate::sample_sources::Rating::NEUTRAL)
        .expect("set tag");
    (temp, source, relative_path, absolute_path)
}

#[test]
fn restore_sample_job_reapplies_looped_metadata() {
    let temp = tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative_path = PathBuf::from("loop.wav");
    let absolute_path = source.root.join(&relative_path);
    let backup_path = source.root.join("loop-backup.wav");
    write_test_wav(&backup_path, &[0.0, 0.1, -0.1]);

    let result = run_undo_file_job(
        UndoFileJob::RestoreSample {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            relative_path: relative_path.clone(),
            absolute_path: absolute_path.clone(),
            backup_path: backup_path.clone(),
            tag: crate::sample_sources::Rating::KEEP_1,
            looped: true,
        },
        Arc::new(AtomicBool::new(false)),
        None,
    );

    assert!(
        matches!(
            &result.result,
            Ok(UndoFileOutcome::Restored {
                source_id,
                relative_path: outcome_path,
                tag,
                looped,
                last_played_at,
                ..
            }) if *source_id == source.id
                && *outcome_path == relative_path
                && *tag == crate::sample_sources::Rating::KEEP_1
                && *looped
                && last_played_at.is_none()
        ),
        "restore sample should keep looped metadata: {:?}",
        result.result
    );

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert_eq!(
        db.tag_for_path(&relative_path)
            .expect("lookup restored tag"),
        Some(crate::sample_sources::Rating::KEEP_1)
    );
    assert_eq!(
        db.looped_for_path(&relative_path)
            .expect("lookup restored looped metadata"),
        Some(true)
    );
    assert!(
        absolute_path.exists(),
        "restore should recreate sample file"
    );
}

fn lock_db_until_released(source_root: &Path) -> (Sender<()>, Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(crate::sample_sources::db::DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).expect("open sqlite lock connection");
        conn.execute_batch("BEGIN IMMEDIATE")
            .expect("start immediate transaction");
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().expect("wait for sqlite lock");
    (lock_release_tx, lock_done_rx)
}

fn release_db_lock(lock_release_tx: Sender<()>, lock_done_rx: Receiver<()>) {
    let _ = lock_release_tx.send(());
    lock_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("wait for sqlite lock release");
}
