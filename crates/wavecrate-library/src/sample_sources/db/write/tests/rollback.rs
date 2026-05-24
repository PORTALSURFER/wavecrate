use std::path::Path;

use tempfile::tempdir;

use super::super::super::{Rating, SourceDatabase, SourceDbError};
use super::helpers::{revision_value, row_snapshot};

#[test]
fn failed_write_wrappers_leave_rows_and_revision_unchanged() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let before_rows = row_snapshot(&db);
    let before_revision = revision_value(&db);

    let err = db.set_looped(Path::new("../escape.wav"), true).unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));

    let after_revision = revision_value(&db);
    assert_eq!(row_snapshot(&db), before_rows);
    assert_eq!(before_revision, after_revision);
}

#[test]
fn batch_errors_roll_back_prior_mutations() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
    let before = row_snapshot(&db);
    let before_revision = revision_value(&db);

    let mut batch = db.write_batch().unwrap();
    batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
    let err = batch
        .set_missing(Path::new("../escape.wav"), true)
        .unwrap_err();
    assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
    drop(batch);

    assert_eq!(row_snapshot(&db), before);
    assert_eq!(revision_value(&db), before_revision);
}
