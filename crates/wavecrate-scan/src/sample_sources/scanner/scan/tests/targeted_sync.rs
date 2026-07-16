use super::*;
use crate::sample_sources::scanner::{sync_paths, sync_paths_with_progress};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn targeted_sync_updates_only_requested_file() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    std::fs::write(dir.path().join("two.wav"), b"two").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::write(dir.path().join("one.wav"), b"changed").unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("one.wav")]).unwrap();

    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.updated, 1);
    assert_eq!(stats.content_changed, 1);
    assert_eq!(db.list_files().unwrap().len(), 2);
    assert_eq!(stats.committed_delta.changed.len(), 1);
    assert!(stats.committed_delta.revision > 0);
}

#[test]
fn targeted_sync_detects_same_size_edit_with_restored_mtime() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("same.wav");
    std::fs::write(&path, b"one").unwrap();
    let original_modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let original_hash = db
        .entry_for_path(Path::new("same.wav"))
        .unwrap()
        .unwrap()
        .content_hash
        .unwrap();

    std::fs::write(&path, b"two").unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
        .unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("same.wav")]).unwrap();
    let current_hash = db
        .entry_for_path(Path::new("same.wav"))
        .unwrap()
        .unwrap()
        .content_hash
        .unwrap();

    assert_ne!(current_hash, original_hash);
    assert_eq!(stats.content_changed, 1);
    assert_eq!(stats.committed_delta.changed.len(), 1);
    assert!(stats.committed_delta.created.is_empty());
}

#[test]
fn targeted_sync_exactly_hashes_an_existing_large_file_edit() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("large.wav");
    std::fs::write(&path, vec![1_u8; 9 * 1024 * 1024]).unwrap();
    let original_modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    complete_pending_deep_hash_for_path(&db, Path::new("large.wav"), None).unwrap();
    let original_hash = db
        .entry_for_path(Path::new("large.wav"))
        .unwrap()
        .unwrap()
        .content_hash
        .unwrap();

    std::fs::write(&path, vec![2_u8; 9 * 1024 * 1024]).unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(original_modified))
        .unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("large.wav")]).unwrap();
    let current_hash = db
        .entry_for_path(Path::new("large.wav"))
        .unwrap()
        .unwrap()
        .content_hash
        .unwrap();

    assert_ne!(current_hash, original_hash);
    assert_eq!(stats.committed_delta.changed.len(), 1);
    assert_eq!(stats.hashes_pending, 0);
}

#[test]
fn targeted_sync_hides_confirmed_missing_file() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    std::fs::write(dir.path().join("two.wav"), b"two").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::remove_file(dir.path().join("one.wav")).unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("one.wav")]).unwrap();

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.missing, 1);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(stats.committed_delta.deleted.len(), 1);
}

#[test]
fn targeted_sync_prunes_removed_folder_prefix() {
    let dir = tempdir().unwrap();
    let drums = dir.path().join("drums");
    std::fs::create_dir_all(&drums).unwrap();
    std::fs::write(drums.join("one.wav"), b"one").unwrap();
    std::fs::write(drums.join("two.wav"), b"two").unwrap();
    std::fs::write(dir.path().join("keep.wav"), b"keep").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::remove_dir_all(&drums).unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("drums")]).unwrap();

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.missing, 2);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("keep.wav"));
}

#[test]
fn targeted_sync_adds_new_file_inside_requested_folder() {
    let dir = tempdir().unwrap();
    let drums = dir.path().join("drums");
    std::fs::create_dir_all(&drums).unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::write(drums.join("kick.wav"), b"kick").unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("drums")]).unwrap();

    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.added, 1);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("drums/kick.wav"));
    assert_eq!(stats.committed_delta.created.len(), 1);
}

#[test]
fn targeted_sync_does_not_claim_unrelated_missing_rename_source() {
    let dir = tempdir().unwrap();
    let unrelated = dir.path().join("unrelated.wav");
    std::fs::write(&unrelated, b"same").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    db.set_tag(Path::new("unrelated.wav"), Rating::KEEP_1)
        .unwrap();

    std::fs::remove_file(&unrelated).unwrap();
    std::fs::write(dir.path().join("requested.wav"), b"same").unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("requested.wav")]).unwrap();

    assert_eq!(stats.renames_reconciled, 0);
    assert_eq!(stats.added, 1);
    assert!(
        db.entry_for_path(Path::new("unrelated.wav"))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        db.entry_for_path(Path::new("requested.wav"))
            .unwrap()
            .unwrap()
            .tag,
        Rating::NEUTRAL
    );
}

#[test]
fn targeted_sync_ignores_appledouble_sidecars() {
    let dir = tempdir().unwrap();
    let drums = dir.path().join("drums");
    std::fs::create_dir_all(&drums).unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::write(drums.join("kick.wav"), b"kick").unwrap();
    std::fs::write(drums.join("._kick.wav"), b"sidecar").unwrap();
    let stats = sync_paths(&db, &[PathBuf::from("drums")]).unwrap();

    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.added, 1);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("drums/kick.wav"));
}

#[test]
fn targeted_sync_cancels_after_a_committed_batch_and_resumes_safely() {
    let dir = tempdir().unwrap();
    let drums = dir.path().join("drums");
    std::fs::create_dir_all(&drums).unwrap();
    for index in 0..70 {
        std::fs::write(drums.join(format!("sample-{index:03}.wav")), b"x").unwrap();
    }
    let db = SourceDatabase::open(dir.path()).unwrap();
    let cancel = AtomicBool::new(false);
    let targets = [PathBuf::from("drums")];

    let result = sync_paths_with_progress(&db, &targets, Some(&cancel), &mut |count, _| {
        if count == 65 {
            cancel.store(true, Ordering::Relaxed);
        }
    });

    assert!(matches!(result, Err(ScanError::Canceled)));
    assert_eq!(db.count_files().unwrap(), 64);

    cancel.store(false, Ordering::Relaxed);
    let resumed = sync_paths_with_progress(&db, &targets, Some(&cancel), &mut |_, _| {})
        .expect("targeted sync must resume from the committed checkpoint");
    assert_eq!(resumed.total_files, 70);
    assert_eq!(db.count_files().unwrap(), 70);
}
