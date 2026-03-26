use super::*;
use crate::app::controller::library::source_folders::delete_recovery::{
    DeleteRecoveryAction, DeleteRecoveryStatus, mark_delete_retained, stage_folder_for_delete,
};
use crate::sample_sources::SampleSource;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

use super::super::journal::{DeleteJournalStage, update_entry_stage};

#[test]
fn unique_restore_path_avoids_collisions() {
    let dir = tempdir().unwrap();
    let original = dir.path().join("folder");
    fs::create_dir_all(&original).unwrap();
    let (target, detail) = unique_restore_path(&original);
    assert_ne!(target, original);
    assert!(detail.is_some());
}

#[test]
fn recover_restores_intent_entry_when_staged_folder_exists() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    update_entry_stage(&staging_root, &staged.id, DeleteJournalStage::Intent)?;

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(original.is_dir());
    assert!(!staging_root.exists());
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Completed,
    );
    Ok(())
}

#[test]
fn recover_treats_missing_staged_folder_as_already_restored() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    update_entry_stage(&staging_root, &staged.id, DeleteJournalStage::Intent)?;
    fs::rename(&staged.staged_absolute, &original).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(original.is_dir());
    assert!(!staging_root.exists());
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Already restored")
    );
    Ok(())
}

#[test]
fn recover_keeps_deleted_entry_staged() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    mark_delete_retained(&staging_root, &staged.id)?;

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!original.exists());
    assert!(staged.staged_absolute.exists());
    assert!(staging_root.exists());
    assert!(report.entries.is_empty());
    Ok(())
}

#[test]
fn recover_cleans_retained_entry_when_folder_was_already_restored() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    mark_delete_retained(&staging_root, &staged.id)?;
    fs::rename(&staged.staged_absolute, &original).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(original.is_dir());
    assert!(!staging_root.exists());
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Completed,
    );
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Already restored")
    );
    Ok(())
}

#[test]
fn recover_restores_unjournaled_staged_folder() {
    let (_temp, source) = sample_source();
    let original = source.root.join("ghost");
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    fs::create_dir_all(staging_root.join("ghost")).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(original.is_dir());
    assert!(!staging_root.exists());
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Completed,
    );
}

#[test]
fn recover_uses_restore_suffix_when_original_exists() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let _staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    fs::create_dir_all(&original).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));
    let restored = source.root.join("gone.restored-1");

    assert!(original.is_dir());
    assert!(restored.is_dir());
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some(format!("Restored as {}", restored.display()).as_str())
    );
    Ok(())
}

#[test]
fn recover_reports_failed_restore_when_staged_folder_is_missing() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"))?;
    fs::remove_dir_all(&staged.staged_absolute).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!original.exists());
    assert!(staging_root.exists());
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Restore,
        DeleteRecoveryStatus::Failed,
    );
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Staged folder missing")
    );
    Ok(())
}

fn sample_source() -> (tempfile::TempDir, SampleSource) {
    let dir = tempdir().unwrap();
    let root = dir.path().join("source");
    fs::create_dir_all(&root).unwrap();
    (dir, SampleSource::new(root))
}

fn assert_recovery(
    entry: &DeleteRecoveryEntry,
    action: DeleteRecoveryAction,
    status: DeleteRecoveryStatus,
) {
    assert_eq!(entry.action, action);
    assert_eq!(entry.status, status);
}
