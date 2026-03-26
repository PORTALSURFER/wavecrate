use crate::app::controller::library::source_folders::delete_recovery::{
    DELETE_STAGING_DIR, DeleteStagingInfo, stage_folder_for_delete,
};
use crate::app::controller::test_support::{dummy_controller, write_test_wav};
use crate::app::state::RetainedFolderDeleteEntry as UiRetainedFolderDeleteEntry;
use crate::sample_sources::{Rating, WavEntry};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[test]
fn retained_restore_keeps_existing_metadata_for_identical_files() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let deleted_entries = vec![entry("Pack/kick.wav", Rating::KEEP_3, 11)];
    let original = source.root.join("Pack");
    fs::create_dir_all(&original).unwrap();
    write_test_wav(&original.join("kick.wav"), &[0.0, 0.2]);
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("Pack"),
        &deleted_entries,
    )?;
    fs::create_dir_all(&original).unwrap();
    write_test_wav(&original.join("kick.wav"), &[0.0, 0.2]);
    controller
        .restore_folder_entries_in_db(&source, &[entry("Pack/kick.wav", Rating::TRASH_1, 99)])?;

    let ui_entry = retained_entry(&source, &staged, deleted_entries);
    let mut scan_sources = HashSet::new();
    controller.restore_retained_folder_delete(&ui_entry, &mut scan_sources)?;

    let db = source.open_db().map_err(|err| err.to_string())?;
    let restored = db
        .entry_for_path(Path::new("Pack/kick.wav"))
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing restored DB entry".to_string())?;
    assert_eq!(restored.tag, Rating::TRASH_1);
    assert_eq!(restored.last_played_at, Some(99));
    assert!(scan_sources.is_empty());
    Ok(())
}

#[test]
fn retained_restore_preserves_older_existing_metadata_on_timestamped_backup() -> Result<(), String>
{
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let older_existing = source.root.join("older.wav");
    write_test_wav(&older_existing, &[0.3, -0.3]);
    thread::sleep(Duration::from_millis(20));
    let deleted_entries = vec![entry("Pack/kick.wav", Rating::KEEP_3, 11)];
    let original = source.root.join("Pack");
    fs::create_dir_all(&original).unwrap();
    write_test_wav(&original.join("kick.wav"), &[0.0, 0.2]);
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("Pack"),
        &deleted_entries,
    )?;
    fs::create_dir_all(&original).unwrap();
    fs::rename(&older_existing, original.join("kick.wav")).unwrap();
    controller
        .restore_folder_entries_in_db(&source, &[entry("Pack/kick.wav", Rating::TRASH_1, 99)])?;

    let ui_entry = retained_entry(&source, &staged, deleted_entries.clone());
    let mut scan_sources = HashSet::new();
    controller.restore_retained_folder_delete(&ui_entry, &mut scan_sources)?;

    let db = source.open_db().map_err(|err| err.to_string())?;
    let canonical = db
        .entry_for_path(Path::new("Pack/kick.wav"))
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing canonical restored DB entry".to_string())?;
    assert_eq!(canonical.tag, Rating::KEEP_3);
    assert_eq!(canonical.last_played_at, Some(11));
    assert!(scan_sources.contains(&source.id));

    let relocated_path = find_timestamped_backup(&source.root.join("Pack"), "kick.replaced-")?;
    let relocated_entry = db
        .entry_for_path(&relocated_path)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing relocated conflict DB entry".to_string())?;
    assert_eq!(relocated_entry.tag, Rating::TRASH_1);
    assert_eq!(relocated_entry.last_played_at, Some(99));
    Ok(())
}

#[test]
fn retained_restore_after_restart_does_not_create_undo_history() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let deleted_entries = vec![entry("Pack/kick.wav", Rating::KEEP_3, 11)];
    let original = source.root.join("Pack");
    fs::create_dir_all(&original).unwrap();
    write_test_wav(&original.join("kick.wav"), &[0.0, 0.2]);
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("Pack"),
        &deleted_entries,
    )?;
    let ui_entry = retained_entry(&source, &staged, deleted_entries);
    controller
        .ui
        .sources
        .folders
        .delete_recovery
        .retained_entries = vec![ui_entry];

    controller.start_restore_retained_folder_deletes();
    assert!(controller.apply_pending_folder_delete_recovery_prompt());
    assert!(source.root.join("Pack/kick.wav").is_file());

    controller.undo();

    assert_eq!(controller.ui.status.text, "Nothing to undo");
    assert!(source.root.join("Pack/kick.wav").is_file());
    Ok(())
}

fn retained_entry(
    source: &crate::sample_sources::SampleSource,
    staged: &DeleteStagingInfo,
    deleted_entries: Vec<WavEntry>,
) -> UiRetainedFolderDeleteEntry {
    UiRetainedFolderDeleteEntry {
        id: staged.id.clone(),
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        source_label: "source".into(),
        relative_path: staged.original_relative.clone(),
        staged_relative: staged.staged_relative.clone(),
        deleted_entries,
    }
}

fn entry(relative: &str, tag: Rating, last_played_at: i64) -> WavEntry {
    WavEntry {
        relative_path: PathBuf::from(relative),
        file_size: 128,
        modified_ns: 9,
        content_hash: Some(format!("hash-{tag:?}-{last_played_at}")),
        tag,
        looped: false,
        locked: tag == Rating::KEEP_3,
        missing: false,
        last_played_at: Some(last_played_at),
    }
}

fn find_timestamped_backup(folder: &Path, prefix: &str) -> Result<PathBuf, String> {
    let entry = fs::read_dir(folder)
        .map_err(|err| err.to_string())?
        .flatten()
        .find(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .ok_or_else(|| format!("Missing timestamped backup with prefix {prefix}"))?;
    Ok(PathBuf::from("Pack").join(entry.file_name()))
}
