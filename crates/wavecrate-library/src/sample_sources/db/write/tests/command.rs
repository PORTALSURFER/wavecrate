use std::path::Path;

use tempfile::tempdir;

use super::super::super::{
    Rating, SourceContentHashWrite, SourceDatabase, SourceFileWrite, SourceTagWrite,
    SourceWriteCommand,
};
use super::helpers::{revision_value, row_snapshot, wav_paths_revision_value};

fn upsert(path: &Path) -> SourceWriteCommand<'_> {
    SourceWriteCommand::UpsertFile(SourceFileWrite {
        relative_path: path,
        file_size: 10,
        modified_ns: 5,
        content_hash: SourceContentHashWrite::Set("hash"),
        tag: SourceTagWrite::Preserve,
        missing: false,
    })
}

#[test]
fn typed_commands_share_one_off_and_batch_semantics() {
    let direct_dir = tempdir().unwrap();
    let direct = SourceDatabase::open_for_source_write(direct_dir.path()).unwrap();
    direct.execute_write(upsert(Path::new("one.wav"))).unwrap();
    direct
        .execute_write(SourceWriteCommand::SetTag {
            path: Path::new("one.wav"),
            tag: Rating::KEEP_1,
        })
        .unwrap();

    let batch_dir = tempdir().unwrap();
    let batched = SourceDatabase::open_for_source_write(batch_dir.path()).unwrap();
    let mut batch = batched.write_batch().unwrap();
    batch.execute_write(upsert(Path::new("one.wav"))).unwrap();
    batch
        .execute_write(SourceWriteCommand::SetTag {
            path: Path::new("one.wav"),
            tag: Rating::KEEP_1,
        })
        .unwrap();
    batch.commit().unwrap();

    assert_eq!(row_snapshot(&direct), row_snapshot(&batched));
    assert_eq!(revision_value(&direct), 2);
    assert_eq!(revision_value(&batched), 1);
    assert_eq!(wav_paths_revision_value(&direct), 1);
    assert_eq!(wav_paths_revision_value(&batched), 1);
}

#[test]
fn metadata_commands_do_not_dirty_the_wav_path_revision() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open_for_source_write(dir.path()).unwrap();
    db.execute_write(upsert(Path::new("one.wav"))).unwrap();

    let mut batch = db.write_batch().unwrap();
    batch
        .execute_write(SourceWriteCommand::SetMissing {
            path: Path::new("one.wav"),
            missing: true,
        })
        .unwrap();
    batch
        .execute_write(SourceWriteCommand::SetMetadata {
            key: "probe",
            value: "complete",
        })
        .unwrap();
    batch.commit().unwrap();

    assert_eq!(revision_value(&db), 2);
    assert_eq!(wav_paths_revision_value(&db), 1);
    assert_eq!(
        db.get_metadata("probe").unwrap().as_deref(),
        Some("complete")
    );
}
