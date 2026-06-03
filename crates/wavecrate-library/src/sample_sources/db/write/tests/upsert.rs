use std::path::Path;

use tempfile::tempdir;

use super::super::super::{Rating, SampleCollection, SourceDatabase};
use super::helpers::revision_value;

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
fn wav_upsert_variants_preserve_hash_tag_and_missing_contracts() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(Path::new("one.wav"), 10, 5, "hash-a", Rating::KEEP_1, true)
        .unwrap();
    batch.commit().unwrap();

    let first = db.list_files().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].content_hash.as_deref(), Some("hash-a"));
    assert_eq!(first[0].tag, Rating::KEEP_1);
    assert!(first[0].missing);

    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_without_hash(Path::new("one.wav"), 12, 6)
        .unwrap();
    batch.commit().unwrap();

    let second = db.list_files().unwrap();
    assert_eq!(second[0].content_hash, None);
    assert_eq!(second[0].tag, Rating::KEEP_1);
    assert!(!second[0].missing);
}

#[test]
fn collections_can_accumulate_multiple_memberships() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let first = SampleCollection::new(0).unwrap();
    let second = SampleCollection::new(1).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch.add_collection(Path::new("one.wav"), second).unwrap();
    batch.add_collection(Path::new("one.wav"), first).unwrap();
    batch.add_collection(Path::new("one.wav"), second).unwrap();
    batch.commit().unwrap();

    assert_eq!(
        db.collections_for_path(Path::new("one.wav")).unwrap(),
        vec![first, second]
    );
    assert_eq!(
        db.collection_for_path(Path::new("one.wav")).unwrap(),
        Some(first)
    );
}

#[test]
fn set_collection_replaces_all_collection_memberships() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let first = SampleCollection::new(0).unwrap();
    let second = SampleCollection::new(1).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch.add_collection(Path::new("one.wav"), first).unwrap();
    batch.add_collection(Path::new("one.wav"), second).unwrap();
    batch
        .set_collection(Path::new("one.wav"), Some(second))
        .unwrap();
    batch.commit().unwrap();

    assert_eq!(
        db.collections_for_path(Path::new("one.wav")).unwrap(),
        vec![second]
    );
}

#[test]
fn metadata_written_inside_batch_commits_with_other_changes() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    batch.set_metadata("custom_key", "custom_value").unwrap();
    batch.commit().unwrap();

    let metadata = db.get_metadata("custom_key").unwrap();
    assert_eq!(metadata.as_deref(), Some("custom_value"));
    assert_eq!(revision_value(&db), 1);
}
