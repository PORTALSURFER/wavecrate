use super::*;
use crate::app::controller::library::source_folders::delete_recovery::stage_folder_for_delete;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn retained_restore_reuses_identical_existing_file() -> Result<(), String> {
    let (_temp, root) = sample_source_root()?;
    let original = root.join("Pack");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"same").unwrap();
    let staging_root = root.join(super::super::DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("Pack"), &[])?;
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"same").unwrap();

    let report = restore_retained_folder_with_merge_at(
        &staged,
        &root,
        &original,
        &staging_root,
        "20260326T105355Z",
    )?;

    assert_eq!(
        report.restored_record_for(Path::new("Pack/kick.wav")),
        Some(&RestoredFileRecord {
            original_relative: PathBuf::from("Pack/kick.wav"),
            final_relative: PathBuf::from("Pack/kick.wav"),
            disposition: RestoredFileDisposition::ReusedExisting,
        })
    );
    assert!(!report.had_conflicts);
    assert_eq!(fs::read(original.join("kick.wav")).unwrap(), b"same");
    assert!(!staging_root.exists());
    Ok(())
}

#[test]
fn retained_restore_keeps_newer_existing_file_and_timestamps_staged_copy() -> Result<(), String> {
    let (_temp, root) = sample_source_root()?;
    let original = root.join("Pack");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"older-staged").unwrap();
    let staging_root = root.join(super::super::DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("Pack"), &[])?;
    thread::sleep(Duration::from_millis(20));
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"newer-existing").unwrap();

    let report = restore_retained_folder_with_merge_at(
        &staged,
        &root,
        &original,
        &staging_root,
        "20260326T105355Z",
    )?;
    let conflict_copy = original.join("kick.recovered-20260326T105355Z.wav");

    assert_eq!(
        fs::read(original.join("kick.wav")).unwrap(),
        b"newer-existing"
    );
    assert_eq!(fs::read(&conflict_copy).unwrap(), b"older-staged");
    assert_eq!(
        report
            .restored_record_for(Path::new("Pack/kick.wav"))
            .map(|record| (&record.final_relative, record.disposition)),
        Some((
            &PathBuf::from("Pack/kick.recovered-20260326T105355Z.wav"),
            RestoredFileDisposition::RestoredTimestamped,
        ))
    );
    assert!(report.had_conflicts);
    Ok(())
}

#[test]
fn retained_restore_replaces_older_existing_file_when_staged_copy_is_newer() -> Result<(), String> {
    let (_temp, root) = sample_source_root()?;
    let original = root.join("Pack");
    let older_file = root.join("older.wav");
    fs::write(&older_file, b"older-existing").unwrap();
    thread::sleep(Duration::from_millis(20));
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"newer-staged").unwrap();
    let staging_root = root.join(super::super::DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("Pack"), &[])?;
    fs::create_dir_all(&original).unwrap();
    fs::rename(&older_file, original.join("kick.wav")).unwrap();

    let report = restore_retained_folder_with_merge_at(
        &staged,
        &root,
        &original,
        &staging_root,
        "20260326T105355Z",
    )?;
    let replaced_copy = original.join("kick.replaced-20260326T105355Z.wav");

    assert_eq!(
        fs::read(original.join("kick.wav")).unwrap(),
        b"newer-staged"
    );
    assert_eq!(fs::read(&replaced_copy).unwrap(), b"older-existing");
    assert_eq!(
        report
            .restored_record_for(Path::new("Pack/kick.wav"))
            .map(|record| (&record.final_relative, record.disposition)),
        Some((
            &PathBuf::from("Pack/kick.wav"),
            RestoredFileDisposition::RestoredCanonical
        ))
    );
    assert_eq!(
        report.existing_relocations,
        vec![ExistingFileRelocation {
            original_relative: PathBuf::from("Pack/kick.wav"),
            relocated_relative: PathBuf::from("Pack/kick.replaced-20260326T105355Z.wav"),
        }]
    );
    assert!(report.had_conflicts);
    Ok(())
}

fn sample_source_root() -> Result<(tempfile::TempDir, PathBuf), String> {
    let dir = tempdir().map_err(|err| err.to_string())?;
    let root = dir.path().join("source");
    fs::create_dir_all(&root).map_err(|err| err.to_string())?;
    Ok((dir, root))
}
