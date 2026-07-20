use super::super::{SampleSource, SourceDatabase};
use super::*;
use crate::sample_sources::{Rating, SourceId};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::tempdir;

fn make_test_db(dir: &Path, filename: &str) -> SourceDatabase {
    let db = SourceDatabase::open_for_test_fixture_source_write(dir).unwrap();
    db.upsert_file(Path::new(filename), 123, 456).unwrap();
    db.set_tag(Path::new(filename), Rating::TRASH_3).unwrap();
    db
}

#[test]
fn rollback_on_failure() {
    let dir = tempdir().unwrap();
    let source_root = dir.path().to_path_buf();
    let db = make_test_db(&source_root, "fail.wav");

    let source = SampleSource::new_with_id(SourceId::new(), source_root.clone());

    let trash_root = dir.path().join("trash");
    let cancel = Arc::new(AtomicBool::new(false));

    let finished = run_trash_move_task_with_progress(
        vec![source],
        trash_root,
        cancel,
        |_| {},
        |_source, _entry, _root| Err("Simulated IO Error".to_string()),
    );

    assert!(!finished.errors.is_empty());

    let files = db.list_files().unwrap();
    assert_eq!(files.len(), 1);
    assert!(
        !files[0].missing,
        "Should rollback missing status on failure"
    );
}

#[test]
fn success_removes_from_db() {
    let dir = tempdir().unwrap();
    let source_root = dir.path().to_path_buf();
    let db = make_test_db(&source_root, "success.wav");

    let source = SampleSource::new_with_id(SourceId::new(), source_root.clone());

    let trash_root = dir.path().join("trash");
    let cancel = Arc::new(AtomicBool::new(false));

    let finished = run_trash_move_task_with_progress(
        vec![source],
        trash_root,
        cancel,
        |_| {},
        |_source, _entry, _root| Ok(()),
    );

    assert!(finished.errors.is_empty());

    let files = db.list_files().unwrap();
    assert_eq!(files.len(), 0, "Should remove file from DB on success");
}

#[test]
fn post_move_remove_failure_refreshes_source_and_keeps_missing_row() {
    let dir = tempdir().unwrap();
    let source_root = dir.path().to_path_buf();
    let db = make_test_db(&source_root, "locked.wav");
    fs::write(source_root.join("locked.wav"), b"wav").unwrap();

    let source = SampleSource::new_with_id(SourceId::new(), source_root.clone());
    let source_id = source.id.clone();

    let trash_root = dir.path().join("trash");
    let cancel = Arc::new(AtomicBool::new(false));
    let mut write_lock = None;

    let finished = run_trash_move_task_with_progress(
        vec![source],
        trash_root,
        cancel,
        |_| {},
        |source, entry, root| {
            move_to_trash(source, entry, root)?;
            let conn = SourceDatabase::open_connection_for_background_job(&source.root)
                .map_err(|err| err.to_string())?;
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|err| err.to_string())?;
            write_lock = Some(conn);
            Ok(())
        },
    );

    assert_eq!(finished.moved, 1);
    assert_eq!(finished.affected_sources, vec![source_id]);
    assert_eq!(finished.errors.len(), 1);
    assert!(
        finished.errors[0].contains("retained it as missing"),
        "error should explain the retained missing-row state: {:?}",
        finished.errors
    );

    drop(write_lock);

    let files = db.list_files().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path, PathBuf::from("locked.wav"));
    assert!(
        files[0].missing,
        "post-move DB removal failure should not keep showing the moved file as available"
    );
}
