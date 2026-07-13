use super::*;
use crate::sample_sources::scanner::sync_paths;
use std::path::PathBuf;

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
