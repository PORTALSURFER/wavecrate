use super::*;

#[test]
fn reconcile_copy_from_staged_file() {
    let temp = TempDir::new().unwrap();
    let target_root = temp.path().join("target");
    std::fs::create_dir_all(&target_root).unwrap();
    let target_db = SourceDatabase::open(&target_root).unwrap();
    let source_path = temp.path().join("external.wav");
    write_wav(&source_path);
    let target_relative = PathBuf::from("copied.wav");
    let staged_relative = staged_relative_for_target(&target_relative, "copy").unwrap();
    let entry = FileOpJournalEntry::new_copy(
        String::from("copy-test"),
        target_relative.clone(),
        staged_relative.clone(),
        Rating::KEEP_1,
        true,
        true,
        Some(123),
    )
    .unwrap();
    insert_entry(&target_db, &entry).unwrap();
    let staged_absolute = target_root.join(&staged_relative);
    std::fs::copy(&source_path, &staged_absolute).unwrap();
    update_stage(
        &target_db,
        &entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();

    let summary = reconcile_pending_ops(&target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(target_root.join(&target_relative).exists());
    assert_eq!(
        target_db.tag_for_path(&target_relative).unwrap(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        target_db.looped_for_path(&target_relative).unwrap(),
        Some(true)
    );
    assert_eq!(
        target_db.locked_for_path(&target_relative).unwrap(),
        Some(true)
    );
    assert_eq!(
        target_db.last_played_at_for_path(&target_relative).unwrap(),
        Some(123)
    );
    assert_no_journal_entries(&target_db);
}
