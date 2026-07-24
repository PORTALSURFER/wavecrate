use std::path::{Path, PathBuf};

use wavecrate_library::sample_sources::{
    SOURCE_FORMAT_POLICY_VERSION, SourceIndexClassification, SourceIndexDiagnostic,
    SourceIndexEntry,
};

use super::*;
use crate::sample_sources::scanner::scan_fs::force_file_metadata_failure;
use crate::sample_sources::scanner::sync_paths;

#[test]
fn full_scan_persists_typed_index_only_entries_across_restart() {
    let directory = tempdir().unwrap();
    std::fs::write(directory.path().join("supported.wav"), b"wav").unwrap();
    std::fs::write(directory.path().join("unsupported.flac"), b"flac").unwrap();
    std::fs::write(directory.path().join("notes.txt"), b"notes").unwrap();
    std::fs::write(directory.path().join("._sidecar.flac"), b"sidecar").unwrap();

    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    scan_once(&database).unwrap();
    assert_eq!(database.list_files().unwrap().len(), 1);
    assert_eq!(
        typed_paths(&database),
        vec![
            (
                PathBuf::from("notes.txt"),
                SourceIndexClassification::UnsupportedNonAudio,
            ),
            (
                PathBuf::from("unsupported.flac"),
                SourceIndexClassification::UnsupportedAudio,
            ),
        ]
    );
    assert!(
        database
            .set_tag(Path::new("notes.txt"), Rating::KEEP_1)
            .is_err()
    );
    drop(database);

    let reopened = SourceDatabase::open_for_scan(directory.path()).unwrap();
    assert_eq!(
        typed_paths(&reopened),
        vec![
            (
                PathBuf::from("notes.txt"),
                SourceIndexClassification::UnsupportedNonAudio,
            ),
            (
                PathBuf::from("unsupported.flac"),
                SourceIndexClassification::UnsupportedAudio,
            ),
        ]
    );
}

#[test]
fn full_scan_reconciles_index_only_change_move_and_delete() {
    let directory = tempdir().unwrap();
    let original = directory.path().join("notes.txt");
    std::fs::write(&original, b"one").unwrap();
    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    scan_once(&database).unwrap();
    let first = database.list_source_index_entries().unwrap().remove(0);

    std::fs::write(&original, b"longer").unwrap();
    scan_once(&database).unwrap();
    let changed = database.list_source_index_entries().unwrap().remove(0);
    assert_eq!(changed.file_size, Some(6));
    assert_eq!(changed.classification, first.classification);

    let moved = directory.path().join("moved.txt");
    std::fs::rename(&original, &moved).unwrap();
    scan_once(&database).unwrap();
    assert_eq!(
        typed_paths(&database),
        vec![(
            PathBuf::from("moved.txt"),
            SourceIndexClassification::UnsupportedNonAudio,
        )]
    );

    std::fs::remove_file(moved).unwrap();
    scan_once(&database).unwrap();
    assert!(database.list_source_index_entries().unwrap().is_empty());
}

#[test]
fn targeted_sync_uses_the_same_index_only_classification_and_reconciliation() {
    let directory = tempdir().unwrap();
    let nested = directory.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    scan_once(&database).unwrap();

    std::fs::write(nested.join("loop.mp3"), b"mp3").unwrap();
    std::fs::write(nested.join("notes.md"), b"notes").unwrap();
    sync_paths(&database, &[PathBuf::from("nested")]).unwrap();
    assert_eq!(
        typed_paths(&database),
        vec![
            (
                PathBuf::from("nested/loop.mp3"),
                SourceIndexClassification::UnsupportedAudio,
            ),
            (
                PathBuf::from("nested/notes.md"),
                SourceIndexClassification::UnsupportedNonAudio,
            ),
        ]
    );

    std::fs::remove_file(nested.join("loop.mp3")).unwrap();
    std::fs::rename(nested.join("notes.md"), nested.join("moved.md")).unwrap();
    sync_paths(&database, &[PathBuf::from("nested")]).unwrap();
    let entries = database.list_source_index_entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].relative_path, Path::new("nested/moved.md"));

    std::fs::write(nested.join("moved.md"), b"longer").unwrap();
    sync_paths(&database, &[PathBuf::from("nested/moved.md")]).unwrap();
    let entries = database.list_source_index_entries().unwrap();
    assert_eq!(entries[0].file_size, Some(6));

    std::fs::remove_file(nested.join("moved.md")).unwrap();
    sync_paths(&database, &[PathBuf::from("nested/moved.md")]).unwrap();
    assert!(database.list_source_index_entries().unwrap().is_empty());
}

#[test]
fn uncertain_subtree_does_not_false_delete_index_only_rows() {
    use crate::sample_sources::scanner::scan_fs::force_directory_read_failure;

    let directory = tempdir().unwrap();
    let protected = directory.path().join("protected");
    std::fs::create_dir(&protected).unwrap();
    std::fs::write(protected.join("notes.txt"), b"notes").unwrap();
    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    scan_once(&database).unwrap();

    std::fs::remove_file(protected.join("notes.txt")).unwrap();
    let failure = force_directory_read_failure(&protected);
    assert!(matches!(
        scan_once(&database),
        Err(ScanError::Incomplete { .. })
    ));
    assert_eq!(
        database.list_source_index_entries().unwrap()[0].relative_path,
        Path::new("protected/notes.txt")
    );

    drop(failure);
    scan_once(&database).unwrap();
    assert!(database.list_source_index_entries().unwrap().is_empty());
}

#[test]
fn inaccessible_observation_is_typed_without_deleting_a_prior_index_row() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("notes.txt");
    std::fs::write(&path, b"notes").unwrap();
    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    scan_once(&database).unwrap();
    assert_eq!(
        database.list_source_index_entries().unwrap()[0].classification,
        SourceIndexClassification::UnsupportedNonAudio
    );

    let failure = force_file_metadata_failure(&path);
    let ScanError::Incomplete { .. } = scan_once(&database).unwrap_err() else {
        panic!("unavailable metadata must leave a retryable scan");
    };
    let inaccessible = database.list_source_index_entries().unwrap().remove(0);
    assert_eq!(
        inaccessible.classification,
        SourceIndexClassification::Inaccessible
    );
    assert_eq!(
        inaccessible.diagnostic,
        Some(SourceIndexDiagnostic::MetadataUnavailable)
    );

    drop(failure);
    scan_once(&database).unwrap();
    let recovered = database.list_source_index_entries().unwrap().remove(0);
    assert_eq!(
        recovered.classification,
        SourceIndexClassification::UnsupportedNonAudio
    );
    assert_eq!(recovered.diagnostic, None);
}

#[test]
fn supported_scan_promotes_a_legacy_index_only_row_without_metadata_inheritance() {
    let directory = tempdir().unwrap();
    let path = Path::new("promoted.wav");
    std::fs::write(directory.path().join(path), b"sample").unwrap();
    let database = SourceDatabase::open_for_scan(directory.path()).unwrap();
    let mut batch = database.write_batch().unwrap();
    batch
        .upsert_source_index_entry(&SourceIndexEntry {
            relative_path: path.to_path_buf(),
            classification: SourceIndexClassification::UnsupportedAudio,
            file_size: Some(6),
            modified_ns: Some(1),
            file_identity: None,
            diagnostic: None,
            format_policy_version: SOURCE_FORMAT_POLICY_VERSION.saturating_sub(1),
        })
        .unwrap();
    batch.commit_auxiliary_state().unwrap();

    scan_once(&database).unwrap();

    assert!(database.list_source_index_entries().unwrap().is_empty());
    let promoted = database.entry_for_path(path).unwrap().unwrap();
    assert_eq!(promoted.tag, Rating::NEUTRAL);
    assert!(!promoted.looped);
    assert!(!promoted.locked);
    assert!(promoted.normal_tags.is_empty());
}

fn typed_paths(database: &SourceDatabase) -> Vec<(PathBuf, SourceIndexClassification)> {
    database
        .list_source_index_entries()
        .unwrap()
        .into_iter()
        .map(|entry| (entry.relative_path, entry.classification))
        .collect()
}
