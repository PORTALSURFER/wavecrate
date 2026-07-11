use super::*;

#[test]
fn scan_add_update_and_prune_missing() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let first = scan_once(&db).unwrap();
    assert_eq!(first.added, 1);
    assert_eq!(first.content_changed, 1);
    assert_eq!(first.changed_samples.len(), 1);
    let initial = db.list_files().unwrap();
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].tag, Rating::NEUTRAL);

    std::fs::write(&file_path, b"longer-data").unwrap();
    let second = scan_once(&db).unwrap();
    assert_eq!(second.updated, 1);
    assert_eq!(second.content_changed, 1);
    assert_eq!(second.changed_samples.len(), 1);

    std::fs::remove_file(&file_path).unwrap();
    let third = scan_once(&db).unwrap();
    assert_eq!(third.missing, 1);
    assert!(db.list_files().unwrap().is_empty());
    let fourth = scan_once(&db).unwrap();
    assert_eq!(fourth.missing, 0);
    assert!(db.list_files().unwrap().is_empty());

    std::fs::write(&file_path, b"one").unwrap();
    let fifth = scan_once(&db).unwrap();
    assert_eq!(fifth.added, 1);
    assert_eq!(fifth.updated, 0);
    assert_eq!(fifth.content_changed, 1);
    assert_eq!(fifth.changed_samples.len(), 1);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].missing);
}

#[test]
fn scan_skips_analysis_when_hash_unchanged() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let first = scan_once(&db).unwrap();
    assert_eq!(first.content_changed, 1);

    std::thread::sleep(Duration::from_millis(2));
    std::fs::write(&file_path, b"one").unwrap();

    let second = scan_once(&db).unwrap();
    assert_eq!(second.updated, 1);
    assert_eq!(second.content_changed, 0);
    assert!(second.changed_samples.is_empty());
}

#[test]
fn scan_adds_duplicate_content_when_original_path_still_exists() {
    let dir = tempdir().unwrap();
    let first_path = dir.path().join("one.wav");
    let duplicate_path = dir.path().join("two.wav");
    std::fs::write(&first_path, b"same-content").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let first = scan_once(&db).unwrap();
    assert_eq!(first.added, 1);

    std::fs::write(&duplicate_path, b"same-content").unwrap();
    let second = scan_once(&db).unwrap();

    assert_eq!(second.added, 1);
    assert_eq!(second.renames_reconciled, 0);
    let mut paths = db
        .list_files()
        .unwrap()
        .into_iter()
        .map(|entry| entry.relative_path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, vec!["one.wav", "two.wav"]);
}

#[test]
fn scan_ignores_non_wav_and_counts_nested() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    std::fs::write(nested.join("two.wav"), b"two").unwrap();
    std::fs::write(dir.path().join("later.aif"), b"aif").unwrap();
    std::fs::write(dir.path().join("later.aiff"), b"aiff").unwrap();
    std::fs::write(dir.path().join("unsupported.flac"), b"flac").unwrap();
    std::fs::write(dir.path().join("unsupported.mp3"), b"mp3").unwrap();
    std::fs::write(dir.path().join("ignore.txt"), b"text").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.added, 2);
    assert_eq!(stats.total_files, 2);
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn scan_in_background_finishes() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    let handle = scan_in_background(dir.path().to_path_buf());
    let stats = handle.join().unwrap().unwrap();
    assert_eq!(stats.added, 1);
}

#[test]
fn scan_with_progress_respects_cancel_flag() {
    use std::sync::atomic::AtomicBool;

    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let cancel = AtomicBool::new(true);
    let mut progress_called = false;
    let result = scan_with_progress(&db, ScanMode::Quick, Some(&cancel), &mut |_, _| {
        progress_called = true;
    });
    assert!(matches!(result, Err(ScanError::Canceled)));
    assert!(!progress_called);
}

#[test]
fn scan_detects_missing_paths_without_double_counting() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::remove_file(&file_path).unwrap();
    let first = scan_once(&db).unwrap();
    assert_eq!(first.missing, 1);

    let second = scan_once(&db).unwrap();
    assert_eq!(second.missing, 0);
}

#[test]
fn scan_detects_changed_content_hash() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();

    std::fs::write(&file_path, b"two").unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.content_changed, 1);
    assert_eq!(stats.changed_samples.len(), 1);
}

#[test]
fn scan_discovery_does_not_hold_the_source_writer() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let mut concurrent_write = None;

    scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if concurrent_write.is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
        let writer = SourceDatabase::open_for_user_metadata_write(dir.path()).unwrap();
        concurrent_write = Some(writer.set_metadata("scan_contention_probe", "complete"));
    })
    .unwrap();

    concurrent_write
        .expect("progress callback must run")
        .expect("metadata writer must complete while discovery is active");
    assert_eq!(
        db.get_metadata("scan_contention_probe").unwrap().as_deref(),
        Some("complete")
    );
}

#[test]
fn scan_revalidation_rejects_file_mutation_after_discovery() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"original").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let before = db.list_files().unwrap().remove(0);
    let mut mutated = false;

    let stale_scan = scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if !mutated {
            std::fs::write(&file_path, b"replacement-with-different-size").unwrap();
            mutated = true;
        }
    })
    .unwrap();

    assert!(mutated);
    assert_eq!(stale_scan.updated, 0);
    assert_eq!(stale_scan.missing, 0);
    let after_stale_scan = db.list_files().unwrap().remove(0);
    assert_eq!(after_stale_scan.file_size, before.file_size);
    assert_eq!(after_stale_scan.modified_ns, before.modified_ns);
    assert_eq!(after_stale_scan.content_hash, before.content_hash);

    let fresh_scan = scan_once(&db).unwrap();
    assert_eq!(fresh_scan.updated, 1);
    assert_ne!(db.list_files().unwrap()[0].file_size, before.file_size);
}
