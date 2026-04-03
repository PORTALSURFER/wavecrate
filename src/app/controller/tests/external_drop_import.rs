use super::super::test_support::{dummy_controller, write_test_wav};
use crate::sample_sources::Rating;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn import_external_files_to_source_folder_copies_into_subfolder_and_db() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let temp = tempdir().unwrap();
    let input_path = temp.path().join("kick.wav");
    write_test_wav(&input_path, &[0.0, 0.25, -0.25]);

    controller
        .import_external_files_to_source_folder_for_tests(
            PathBuf::from("Drums"),
            vec![input_path.clone()],
        )
        .unwrap();

    let expected_relative = PathBuf::from("Drums").join("kick.wav");
    let expected_absolute = source.root.join(&expected_relative);
    assert!(expected_absolute.is_file());

    let db = controller.database_for(&source).unwrap();
    let entries = db.list_files().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.relative_path == expected_relative)
    );
}

#[test]
fn import_external_files_to_source_folder_resets_stale_metadata_for_reused_path() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let expected_relative = PathBuf::from("Drums").join("kick.wav");
    let db = controller.database_for(&source).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            &expected_relative,
            10,
            1,
            "stale-hash",
            Rating::KEEP_3,
            true,
        )
        .unwrap();
    batch.commit().unwrap();
    db.set_looped(&expected_relative, true).unwrap();
    db.set_locked(&expected_relative, true).unwrap();
    db.set_last_played_at(&expected_relative, 77).unwrap();

    let temp = tempdir().unwrap();
    let input_path = temp.path().join("kick.wav");
    write_test_wav(&input_path, &[0.0, 0.25, -0.25]);

    controller
        .import_external_files_to_source_folder_for_tests(PathBuf::from("Drums"), vec![input_path])
        .unwrap();

    let entry = db
        .entry_for_path(Path::new("Drums/kick.wav"))
        .unwrap()
        .expect("imported row");
    assert_eq!(entry.tag, Rating::NEUTRAL);
    assert!(!entry.looped);
    assert!(!entry.locked);
    assert_eq!(entry.last_played_at, None);
    assert!(!entry.missing);
    assert_eq!(entry.content_hash, None);
}
