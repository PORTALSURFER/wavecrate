use super::*;

#[test]
fn scan_detects_rename_and_preserves_tag() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    std::fs::write(&first_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    std::fs::rename(&first_path, &second_path).unwrap();
    let stats = scan_once(&db).unwrap();

    assert_eq!(stats.missing, 0);
    assert_eq!(stats.added, 0);
    assert_eq!(stats.content_changed, 0);
    assert_eq!(stats.updated, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert!(!rows[0].missing);
}

#[test]
fn quick_scan_defers_hash_for_large_file() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("large.wav");
    std::fs::write(&file_path, vec![0u8; 9 * 1024 * 1024]).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.hashes_pending, 1);
    assert_eq!(stats.hashes_computed, 0);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].content_hash.is_none());
}

#[test]
fn quick_scan_reconciles_large_rename_and_preserves_tag() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    std::fs::write(&first_path, vec![0u8; 9 * 1024 * 1024]).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    hard_rescan(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    std::fs::rename(&first_path, &second_path).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.renames_reconciled, 1);
    assert_eq!(stats.hashes_pending, 1);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert!(!rows[0].missing);
    assert!(rows[0].content_hash.is_none());

    let deep_stats = crate::sample_sources::scanner::scan_hash::deep_hash_scan(&db, None).unwrap();
    assert_eq!(deep_stats.hashes_computed, 1);
    assert_eq!(deep_stats.renames_reconciled, 0);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("two.wav"));
    assert_eq!(rows[0].tag, Rating::KEEP_1);
    assert!(!rows[0].missing);
    assert!(rows[0].content_hash.is_some());
}

#[cfg(unix)]
#[test]
fn quick_scan_avoids_ambiguous_large_rename() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let second_path = dir.path().join("two.wav");
    let third_path = dir.path().join("three.wav");
    let payload = vec![0u8; 9 * 1024 * 1024];
    std::fs::write(&first_path, &payload).unwrap();
    std::fs::write(&second_path, &payload).unwrap();

    let timestamp = 1_700_000_000i64;
    set_file_times(&first_path, timestamp, 0);
    set_file_times(&second_path, timestamp, 0);

    let db = SourceDatabase::open(dir.path()).unwrap();
    hard_rescan(&db).unwrap();
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();

    std::fs::remove_file(&first_path).unwrap();
    std::fs::remove_file(&second_path).unwrap();
    std::fs::write(&third_path, &payload).unwrap();
    set_file_times(&third_path, timestamp, 0);

    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.renames_reconciled, 0);
    assert_eq!(stats.added, 1);
    assert_eq!(stats.missing, 2);

    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, Path::new("three.wav"));
    assert_eq!(rows[0].tag, Rating::NEUTRAL);
    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 2);
    assert!(pending
        .iter()
        .any(|entry| entry.relative_path == Path::new("one.wav") && entry.tag == Rating::KEEP_1));
}
