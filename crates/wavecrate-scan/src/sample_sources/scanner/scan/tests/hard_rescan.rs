use super::*;

#[test]
fn hard_rescan_prunes_missing_rows() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::remove_file(&file_path).unwrap();
    scan_once(&db).unwrap();
    assert!(db.list_files().unwrap().is_empty());

    let stats = hard_rescan(&db).unwrap();
    assert_eq!(stats.missing, 0);
    let rows = db.list_files().unwrap();
    assert!(rows.is_empty());
    assert!(db.list_pending_renames().unwrap().is_empty());
}

#[test]
fn hard_rescan_prunes_missing_files_with_tags() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    std::fs::remove_file(&file_path).unwrap();
    scan_once(&db).unwrap();

    let stats = hard_rescan(&db).unwrap();
    assert_eq!(stats.missing, 0);
    let rows = db.list_files().unwrap();
    assert!(rows.is_empty());
    assert!(db.list_pending_renames().unwrap().is_empty());
}

#[test]
fn hard_rescan_prunes_missing_without_touching_existing() {
    let dir = tempdir().unwrap();
    let keep_path = dir.path().join("keep.wav");
    let remove_path = dir.path().join("remove.wav");
    std::fs::write(&keep_path, b"keep").unwrap();
    std::fs::write(&remove_path, b"remove").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::remove_file(&remove_path).unwrap();
    let stats = hard_rescan(&db).unwrap();
    assert_eq!(stats.missing, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("keep.wav"));
    assert!(db.list_pending_renames().unwrap().is_empty());
}
