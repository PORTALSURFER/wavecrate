use super::*;

#[test]
fn reconcile_move_from_staged_file() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.staged_absolute()).unwrap();
    update_stage(
        &fixture.target_db,
        &fixture.entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(!fixture.staged_absolute().exists());
    assert!(fixture.target_absolute().exists());
    assert!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap()
            .is_none()
    );
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_same_source_move_from_staged_file() {
    let temp = TempDir::new().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let db = SourceDatabase::open(&source_root).unwrap();
    let source_relative = PathBuf::from("one.wav");
    let source_absolute = source_root.join(&source_relative);
    write_wav(&source_absolute);
    db.upsert_file(&source_relative, 16, 1).unwrap();
    db.set_tag(&source_relative, Rating::KEEP_1).unwrap();
    db.set_looped(&source_relative, true).unwrap();
    db.set_locked(&source_relative, true).unwrap();
    db.set_last_played_at(&source_relative, 123).unwrap();
    let target_relative = PathBuf::from("moved.wav");
    let staged_relative = staged_relative_for_target(&target_relative, "test").unwrap();
    let entry = FileOpJournalEntry::new_move(
        String::from("move-test"),
        MoveJournalEntryInit {
            source_root: source_root.clone(),
            source_relative: source_relative.clone(),
            target_relative: target_relative.clone(),
            staged_relative: staged_relative.clone(),
            tag: Rating::KEEP_1,
            looped: true,
            locked: true,
            last_played_at: Some(123),
        },
    )
    .unwrap();
    insert_entry(&db, &entry).unwrap();
    let staged_absolute = source_root.join(&staged_relative);
    std::fs::rename(&source_absolute, &staged_absolute).unwrap();
    update_stage(&db, &entry.id, FileOpStage::Staged, Some(16), Some(1)).unwrap();

    let summary = reconcile_pending_ops(&db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(!staged_absolute.exists());
    assert!(source_root.join(&target_relative).exists());
    assert!(db.tag_for_path(&source_relative).unwrap().is_none());
    assert_eq!(
        db.tag_for_path(&target_relative).unwrap(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(db.looped_for_path(&target_relative).unwrap(), Some(true));
    assert_eq!(db.locked_for_path(&target_relative).unwrap(), Some(true));
    assert_eq!(
        db.last_played_at_for_path(&target_relative).unwrap(),
        Some(123)
    );
    assert_no_journal_entries(&db);
}

#[test]
fn reconcile_target_db_stage_removes_orphaned_source_row() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.target_absolute()).unwrap();
    fixture.stage_target_db();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.target_absolute().exists());
    assert!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap()
            .is_none()
    );
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_source_db_stage_is_idempotent_after_move_completion() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.target_absolute()).unwrap();
    fixture
        .target_db
        .upsert_file(&fixture.target_relative, 16, 1)
        .unwrap();
    fixture
        .target_db
        .set_tag(&fixture.target_relative, Rating::KEEP_1)
        .unwrap();
    fixture
        .target_db
        .set_looped(&fixture.target_relative, true)
        .unwrap();
    fixture
        .target_db
        .set_locked(&fixture.target_relative, true)
        .unwrap();
    fixture
        .target_db
        .set_last_played_at(&fixture.target_relative, 123)
        .unwrap();
    fixture
        .source_db
        .remove_file(&fixture.source_relative)
        .unwrap();
    fixture.stage_source_db();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.target_absolute().exists());
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_missing_staged_file_keeps_original_source_when_target_missing() {
    let fixture = MoveRecoveryFixture::new();
    update_stage(
        &fixture.target_db,
        &fixture.entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();
    fixture
        .target_db
        .upsert_file(&fixture.target_relative, 16, 1)
        .unwrap();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.source_absolute().exists());
    assert!(!fixture.target_absolute().exists());
    assert_eq!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap(),
        Some(Rating::KEEP_1)
    );
    assert!(
        fixture
            .target_db
            .tag_for_path(&fixture.target_relative)
            .unwrap()
            .is_none()
    );
    assert_no_journal_entries(&fixture.target_db);
}
