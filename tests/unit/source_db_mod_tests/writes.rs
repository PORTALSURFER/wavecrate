use super::*;

#[test]
fn upsert_wrapper_matches_batch_insert_contract() {
    let single_dir = tempdir().unwrap();
    let single = SourceDatabase::open(single_dir.path()).unwrap();
    single.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let batch_dir = tempdir().unwrap();
    let batch_db = SourceDatabase::open(batch_dir.path()).unwrap();
    let mut batch = batch_db.write_batch().unwrap();
    batch.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    batch.commit().unwrap();

    let single_rows = single.list_files().unwrap();
    let batch_rows = batch_db.list_files().unwrap();
    assert_eq!(single_rows.len(), 1);
    assert_eq!(batch_rows.len(), 1);
    assert_eq!(single_rows[0].relative_path, batch_rows[0].relative_path);
    assert_eq!(single_rows[0].file_size, batch_rows[0].file_size);
    assert_eq!(single_rows[0].modified_ns, batch_rows[0].modified_ns);
    assert_eq!(single_rows[0].content_hash, batch_rows[0].content_hash);
    assert_eq!(single_rows[0].tag, batch_rows[0].tag);
    assert_eq!(single_rows[0].looped, batch_rows[0].looped);
    assert_eq!(single_rows[0].locked, batch_rows[0].locked);
    assert_eq!(single_rows[0].missing, batch_rows[0].missing);
    assert_eq!(single_rows[0].last_played_at, batch_rows[0].last_played_at);
    assert_eq!(revision_value(&single), 1);
    assert_eq!(revision_value(&batch_db), 1);
}

#[test]
fn single_write_wrappers_match_batch_results_and_revision_behavior() {
    let single_dir = tempdir().unwrap();
    let single = SourceDatabase::open(single_dir.path()).unwrap();
    single.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    assert_eq!(revision_value(&single), 1);
    single
        .set_tag(Path::new("one.wav"), Rating::KEEP_1)
        .unwrap();
    assert_eq!(revision_value(&single), 2);
    single.set_looped(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 3);
    single.set_locked(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 4);
    single.set_missing(Path::new("one.wav"), true).unwrap();
    assert_eq!(revision_value(&single), 5);
    single.set_last_played_at(Path::new("one.wav"), 42).unwrap();
    assert_eq!(revision_value(&single), 6);

    let batch_dir = tempdir().unwrap();
    let batch_db = SourceDatabase::open(batch_dir.path()).unwrap();
    batch_db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    assert_eq!(revision_value(&batch_db), 1);
    let mut batch = batch_db.write_batch().unwrap();
    batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    batch.set_looped(Path::new("one.wav"), true).unwrap();
    batch.set_locked(Path::new("one.wav"), true).unwrap();
    batch.set_missing(Path::new("one.wav"), true).unwrap();
    batch.set_last_played_at(Path::new("one.wav"), 42).unwrap();
    batch.commit().unwrap();
    assert_eq!(revision_value(&batch_db), 2);

    let single_rows = single.list_files().unwrap();
    let batch_rows = batch_db.list_files().unwrap();
    assert_eq!(single_rows.len(), 1);
    assert_eq!(single_rows[0].relative_path, batch_rows[0].relative_path);
    assert_eq!(single_rows[0].tag, batch_rows[0].tag);
    assert_eq!(single_rows[0].looped, batch_rows[0].looped);
    assert_eq!(single_rows[0].locked, batch_rows[0].locked);
    assert_eq!(single_rows[0].missing, batch_rows[0].missing);
    assert_eq!(single_rows[0].last_played_at, batch_rows[0].last_played_at);

    single.remove_file(Path::new("one.wav")).unwrap();
    assert_eq!(revision_value(&single), 7);
    let mut batch = batch_db.write_batch().unwrap();
    batch.remove_file(Path::new("one.wav")).unwrap();
    batch.commit().unwrap();
    assert_eq!(revision_value(&batch_db), 3);

    assert!(single.list_files().unwrap().is_empty());
    assert!(batch_db.list_files().unwrap().is_empty());
}

#[test]
fn failed_write_wrappers_leave_rows_and_revision_unchanged() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let before_rows = db.list_files().unwrap();
    let before_revision = revision_value(&db);

    let err = db.set_looped(Path::new("../escape.wav"), true).unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
    let after_rows = db.list_files().unwrap();
    let after_revision = revision_value(&db);
    assert_eq!(before_rows.len(), after_rows.len());
    assert_eq!(before_rows[0].relative_path, after_rows[0].relative_path);
    assert_eq!(before_rows[0].looped, after_rows[0].looped);
    assert_eq!(before_revision, after_revision);
}
