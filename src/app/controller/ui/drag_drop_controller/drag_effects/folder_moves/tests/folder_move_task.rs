use super::super::worker::run_folder_move_task;
use super::support::{
    Must, folder_move_request, folder_move_test_guard, lock_db_until_released,
    setup_folder_move_fixture,
};
use crate::sample_sources::{Rating, SourceDatabase};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[test]
/// Moving a folder relocates contained files and rewrites their source DB paths.
fn folder_move_updates_db_entries() {
    let _guard = folder_move_test_guard();
    let (_temp, source, source_root) = setup_folder_move_fixture();
    let request = folder_move_request(&source, &source_root, "old", "dest");
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
    assert_eq!(
        db.looped_for_path(Path::new("dest/old/one.wav")).must(),
        Some(true)
    );
    assert_eq!(
        db.locked_for_path(Path::new("dest/old/one.wav")).must(),
        Some(true)
    );
    assert_eq!(
        db.last_played_at_for_path(Path::new("dest/old/one.wav"))
            .must(),
        Some(42)
    );
}

#[test]
/// Cancelling before folder processing starts leaves filesystem and DB state unchanged.
fn folder_move_cancelled_before_processing_keeps_source_unchanged() {
    let _guard = folder_move_test_guard();
    let (_temp, source, source_root) = setup_folder_move_fixture();
    let request = folder_move_request(&source, &source_root, "old", "dest");
    let result = run_folder_move_task(request, Arc::new(AtomicBool::new(true)), None);

    assert!(result.cancelled);
    assert!(!result.folder_moved);
    assert!(result.moved.is_empty());
    assert!(result.errors.is_empty());
    assert!(source_root.join("old/one.wav").is_file());
    assert!(!source_root.join("dest/old").exists());
    let db = SourceDatabase::open(&source_root).must();
    assert_eq!(
        db.tag_for_path(Path::new("old/one.wav")).must(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        db.locked_for_path(Path::new("old/one.wav")).must(),
        Some(true)
    );
}

#[test]
/// Moving a folder into one of its descendants is rejected without touching the source tree.
fn folder_move_rejects_descendant_target() {
    let _guard = folder_move_test_guard();
    let (_temp, source, source_root) = setup_folder_move_fixture();
    std::fs::create_dir_all(source_root.join("old/child")).must();
    let request = folder_move_request(&source, &source_root, "old", "old/child");
    let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);

    assert!(!result.folder_moved);
    assert!(result.moved.is_empty());
    assert_eq!(
        result.errors,
        vec!["Cannot move a folder into itself".to_string()]
    );
    assert!(source_root.join("old/one.wav").is_file());
    assert!(source_root.join("old/child").is_dir());
}

#[test]
/// An existing destination folder rejects the move before any filesystem rename occurs.
fn folder_move_rejects_existing_destination_folder() {
    let _guard = folder_move_test_guard();
    let (_temp, source, source_root) = setup_folder_move_fixture();
    std::fs::create_dir_all(source_root.join("dest/old")).must();
    let request = folder_move_request(&source, &source_root, "old", "dest");
    let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);

    assert!(!result.folder_moved);
    assert!(result.moved.is_empty());
    assert_eq!(
        result.errors,
        vec![format!(
            "Folder already exists: {}",
            PathBuf::from("dest").join("old").display()
        )]
    );
    assert!(source_root.join("old/one.wav").is_file());
    assert!(source_root.join("dest/old").is_dir());
    let db = SourceDatabase::open(&source_root).must();
    assert_eq!(
        db.tag_for_path(Path::new("old/one.wav")).must(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        db.locked_for_path(Path::new("old/one.wav")).must(),
        Some(true)
    );
}

#[test]
/// A database lock after the filesystem rename rolls the folder back to its original path.
fn folder_move_db_write_failure_rolls_back_source_and_db_state() {
    let _guard = folder_move_test_guard();
    let (_temp, source, source_root) = setup_folder_move_fixture();
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source_root);
    let request = folder_move_request(&source, &source_root, "old", "dest");
    let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);
    let _ = lock_release_tx.send(());
    lock_done_rx.recv_timeout(Duration::from_secs(1)).must();

    assert!(!result.folder_moved);
    assert!(result.moved.is_empty());
    assert_eq!(result.new_folder, PathBuf::from("dest/old"));
    assert!(result.errors.iter().any(|err| {
        err.contains("Failed to start database update")
            || err.contains("Failed to drop old entry")
            || err.contains("Failed to register moved file")
            || err.contains("Failed to copy tag")
            || err.contains("Failed to copy loop marker")
            || err.contains("Failed to copy keep lock")
            || err.contains("Failed to copy playback age")
            || err.contains("Failed to save folder move")
    }));
    assert!(source_root.join("old/one.wav").is_file());
    assert!(!source_root.join("dest/old").exists());
    let db = SourceDatabase::open(&source_root).must();
    assert_eq!(
        db.tag_for_path(Path::new("old/one.wav")).must(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        db.locked_for_path(Path::new("old/one.wav")).must(),
        Some(true)
    );
    assert!(
        db.tag_for_path(Path::new("dest/old/one.wav"))
            .must()
            .is_none()
    );
}
