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

#[test]
fn scan_refreshes_noop_row_after_concurrent_removal() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let mut removed = false;

    scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if removed {
            return;
        }
        let writer = SourceDatabase::open(dir.path()).unwrap();
        let mut batch = writer.write_batch().unwrap();
        batch.remove_file(Path::new("one.wav")).unwrap();
        batch.commit().unwrap();
        removed = true;
    })
    .unwrap();

    assert!(removed);
    assert!(db.entry_for_path(Path::new("one.wav")).unwrap().is_some());
}

#[test]
fn missing_stage_keeps_concurrently_restored_live_row() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("restored.wav");
    std::fs::write(&file_path, b"old").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let stale = db
        .entry_for_path(Path::new("restored.wav"))
        .unwrap()
        .unwrap();
    std::fs::remove_file(&file_path).unwrap();

    std::fs::write(&file_path, b"new-longer").unwrap();
    let facts = super::super::super::scan_fs::read_facts(dir.path(), &file_path).unwrap();
    db.upsert_file(Path::new("restored.wav"), facts.size, facts.modified_ns)
        .unwrap();
    let mut stats = ScanStats::default();
    let mut batch = db.write_batch().unwrap();

    super::super::super::scan_diff::mark_missing(
        &db,
        &mut batch,
        [stale],
        &mut stats,
        ScanMode::Quick,
    )
    .unwrap();
    batch.commit().unwrap();

    assert_eq!(stats.missing, 0);
    let restored = db
        .entry_for_path(Path::new("restored.wav"))
        .unwrap()
        .unwrap();
    assert_eq!(restored.file_size, facts.size);
    assert_eq!(restored.modified_ns, facts.modified_ns);
}

#[test]
fn missing_stage_prunes_path_replaced_by_directory() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("replaced.wav");
    std::fs::write(&file_path, b"old").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    std::fs::remove_file(&file_path).unwrap();
    std::fs::create_dir(&file_path).unwrap();

    let stats = scan_once(&db).unwrap();

    assert_eq!(stats.missing, 1);
    assert!(
        db.entry_for_path(Path::new("replaced.wav"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn scan_rebases_noop_when_concurrent_writer_clears_hash() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let row = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
    let mut cleared = false;

    let stats = scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if cleared {
            return;
        }
        let writer = SourceDatabase::open(dir.path()).unwrap();
        let mut batch = writer.write_batch().unwrap();
        batch
            .upsert_file_without_hash(Path::new("one.wav"), row.file_size, row.modified_ns)
            .unwrap();
        batch.commit().unwrap();
        cleared = true;
    })
    .unwrap();

    assert!(cleared);
    assert_eq!(stats.hashes_computed, 1);
    assert!(
        db.entry_for_path(Path::new("one.wav"))
            .unwrap()
            .unwrap()
            .content_hash
            .is_some()
    );
}

#[test]
fn missing_repair_survives_concurrent_hash_clear() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"one").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let row = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
    db.set_missing(Path::new("one.wav"), true).unwrap();
    let mut cleared = false;

    let stats = scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if cleared {
            return;
        }
        let writer = SourceDatabase::open(dir.path()).unwrap();
        let mut batch = writer.write_batch().unwrap();
        batch
            .upsert_file_without_hash(Path::new("one.wav"), row.file_size, row.modified_ns)
            .unwrap();
        batch.commit().unwrap();
        cleared = true;
    })
    .unwrap();

    assert!(cleared);
    assert_eq!(stats.hashes_computed, 1);
    let repaired = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
    assert!(!repaired.missing);
    assert!(repaired.content_hash.is_some());
}

#[cfg(unix)]
#[test]
fn scan_hashes_current_bytes_when_facts_are_preserved_after_discovery() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("one.wav");
    std::fs::write(&file_path, b"old!").unwrap();
    let timestamp = 1_700_000_000;
    set_file_times(&file_path, timestamp, 0);
    let db = SourceDatabase::open(dir.path()).unwrap();
    let mut replaced = false;

    scan_with_progress(&db, ScanMode::Quick, None, &mut |_, _| {
        if !replaced {
            std::fs::write(&file_path, b"new!").unwrap();
            set_file_times(&file_path, timestamp, 0);
            replaced = true;
        }
    })
    .unwrap();

    let row = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
    assert_eq!(
        row.content_hash.as_deref(),
        Some(blake3::hash(b"new!").to_hex().as_str())
    );
}

#[test]
fn cancellation_after_first_committed_batch_finishes_consistently() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tempdir().unwrap();
    for index in 0..70 {
        std::fs::write(dir.path().join(format!("sample-{index:03}.wav")), b"x").unwrap();
    }
    let db = SourceDatabase::open(dir.path()).unwrap();
    let cancel = AtomicBool::new(false);

    let stats = scan_with_progress(&db, ScanMode::Quick, Some(&cancel), &mut |count, _| {
        if count == 65 {
            cancel.store(true, Ordering::Relaxed);
        }
    })
    .expect("once a bounded batch commits, the scan must finish consistently");

    assert_eq!(stats.total_files, 70);
    assert_eq!(db.count_files().unwrap(), 70);
}

#[test]
fn unchanged_large_scan_only_commits_completion_metadata() {
    let dir = tempdir().unwrap();
    for index in 0..130 {
        std::fs::write(dir.path().join(format!("sample-{index:03}.wav")), b"x").unwrap();
    }
    let db = SourceDatabase::open(dir.path()).unwrap();
    scan_once(&db).unwrap();
    let revision_before = db
        .get_metadata("revision")
        .unwrap()
        .unwrap()
        .parse::<u64>()
        .unwrap();

    let stats = scan_once(&db).unwrap();

    let revision_after = db
        .get_metadata("revision")
        .unwrap()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(stats.updated, 0);
    assert_eq!(stats.added, 0);
    assert_eq!(revision_after, revision_before + 1);
}

#[test]
fn skipped_existing_file_is_not_used_as_a_rename_source() {
    let dir = tempdir().unwrap();
    let hidden = dir.path().join(".hidden");
    std::fs::create_dir(&hidden).unwrap();
    std::fs::write(hidden.join("old.wav"), b"same").unwrap();
    std::fs::write(dir.path().join("new.wav"), b"same").unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    let facts =
        super::super::super::scan_fs::read_facts(dir.path(), &hidden.join("old.wav")).unwrap();
    let hash = blake3::hash(b"same").to_hex().to_string();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash(
            Path::new(".hidden/old.wav"),
            facts.size,
            facts.modified_ns,
            &hash,
        )
        .unwrap();
    batch
        .set_tag(Path::new(".hidden/old.wav"), Rating::KEEP_1)
        .unwrap();
    batch.commit().unwrap();

    let stats = scan_once(&db).unwrap();

    assert_eq!(stats.renames_reconciled, 0);
    let new_entry = db.entry_for_path(Path::new("new.wav")).unwrap().unwrap();
    assert_eq!(new_entry.tag, Rating::NEUTRAL);
}
