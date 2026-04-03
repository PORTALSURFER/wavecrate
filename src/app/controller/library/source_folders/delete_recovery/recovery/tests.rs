use super::*;
use crate::app::controller::library::source_folders::delete_recovery::restore_merge::restore_retained_folder_with_merge_with_stamp;
use crate::app::controller::library::source_folders::delete_recovery::{
    DeleteRecoveryAction, DeleteRecoveryStatus, mark_delete_restore_pending_db,
    mark_delete_retained, stage_folder_for_delete,
};
use crate::sample_sources::SampleSource;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
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
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
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
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
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
    let deleted_entries = vec![sample_entry("gone/kick.wav")];
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("gone"),
        &deleted_entries,
    )?;
    mark_delete_retained(&staging_root, &staged.id)?;

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!original.exists());
    assert!(staged.staged_absolute.exists());
    assert!(staging_root.exists());
    assert!(report.entries.is_empty());
    assert_eq!(report.retained_entries.len(), 1);
    let retained = &report.retained_entries[0];
    assert_eq!(retained.original_relative, Path::new("gone"));
    assert_eq!(retained.deleted_entries.len(), 1);
    assert_eq!(
        retained.deleted_entries[0].relative_path,
        deleted_entries[0].relative_path
    );
    assert_eq!(
        retained.deleted_entries[0].content_hash.as_deref(),
        deleted_entries[0].content_hash.as_deref()
    );
    assert_eq!(
        retained.deleted_entries[0].tag.val(),
        deleted_entries[0].tag.val()
    );
    assert_eq!(
        retained.deleted_entries[0].looped,
        deleted_entries[0].looped
    );
    assert_eq!(
        retained.deleted_entries[0].locked,
        deleted_entries[0].locked
    );
    assert_eq!(
        retained.deleted_entries[0].last_played_at,
        deleted_entries[0].last_played_at
    );
    Ok(())
}

#[test]
fn recover_keeps_retained_entry_available_when_original_folder_reappears() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let deleted_entries = vec![sample_entry("gone/kick.wav")];
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("gone"),
        &deleted_entries,
    )?;
    mark_delete_retained(&staging_root, &staged.id)?;
    fs::create_dir_all(&original).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(original.is_dir());
    assert!(staged.staged_absolute.exists());
    assert!(report.entries.is_empty());
    assert_eq!(report.retained_entries.len(), 1);
    assert_eq!(
        report.retained_entries[0].original_relative,
        Path::new("gone")
    );
    assert_eq!(
        report.retained_entries[0].deleted_entries[0].relative_path,
        deleted_entries[0].relative_path
    );
    Ok(())
}

#[test]
fn recover_cleans_retained_entry_when_folder_was_already_restored() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
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
fn recover_cleans_retained_entry_when_folder_was_already_purged() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
    mark_delete_retained(&staging_root, &staged.id)?;
    fs::remove_dir_all(&staged.staged_absolute).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!original.exists());
    assert!(!staging_root.exists());
    assert_recovery(
        &report.entries[0],
        DeleteRecoveryAction::Finalize,
        DeleteRecoveryStatus::Completed,
    );
    assert_eq!(report.entries[0].detail.as_deref(), Some("Already purged"));
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
fn recover_skips_unjournaled_restore_when_delete_journal_is_unreadable() {
    let (_temp, source) = sample_source();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = staging_root.join("gone");
    fs::create_dir_all(&staged).unwrap();
    fs::write(staging_root.join("delete_journal.json"), b"{broken").unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!source.root.join("gone").exists());
    assert!(staged.is_dir());
    assert!(report.entries.is_empty());
    assert!(report.retained_entries.is_empty());
    assert!(report.scan_sources.is_empty());
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].contains("Failed to read delete journal"));
    assert!(report.errors[0].contains("leaving staged deletes untouched"));
}

#[test]
fn recover_uses_restore_suffix_when_original_exists() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let _staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
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
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
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

#[test]
fn recover_replays_pending_retained_restore_metadata_after_restart() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"staged").unwrap();
    let deleted_entries = vec![sample_entry("gone/kick.wav")];
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("gone"),
        &deleted_entries,
    )?;
    mark_delete_retained(&staging_root, &staged.id)?;
    mark_delete_restore_pending_db(&staging_root, &staged.id, "20260326T105355Z")?;
    restore_retained_folder_with_merge_with_stamp(
        &staged,
        &source.root,
        &source.root.join("gone"),
        &staging_root,
        "20260326T105355Z",
    )?;

    let before = source.open_db().map_err(|err| err.to_string())?;
    assert!(
        before
            .entry_for_path(Path::new("gone/kick.wav"))
            .map_err(|err| err.to_string())?
            .is_none()
    );

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    let db = source.open_db().map_err(|err| err.to_string())?;
    let restored = db
        .entry_for_path(Path::new("gone/kick.wav"))
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing restored DB row".to_string())?;
    assert_eq!(restored.tag, deleted_entries[0].tag);
    assert_eq!(restored.looped, deleted_entries[0].looped);
    assert_eq!(restored.locked, deleted_entries[0].locked);
    assert_eq!(restored.last_played_at, deleted_entries[0].last_played_at);
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Completed retained restore after restart")
    );
    assert!(report.scan_sources.is_empty());
    assert!(!staging_root.exists());
    Ok(())
}

#[test]
fn recover_pending_retained_restore_requests_hard_sync_when_deleted_snapshot_is_empty()
-> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"staged").unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
    mark_delete_retained(&staging_root, &staged.id)?;
    mark_delete_restore_pending_db(&staging_root, &staged.id, "20260326T105355Z")?;
    restore_retained_folder_with_merge_with_stamp(
        &staged,
        &source.root,
        &source.root.join("gone"),
        &staging_root,
        "20260326T105355Z",
    )?;

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(source.root.join("gone/kick.wav").is_file());
    assert_eq!(report.scan_sources, vec![source.id.clone()]);
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Completed retained restore after restart")
    );
    let db = source.open_db().map_err(|err| err.to_string())?;
    assert!(
        db.entry_for_path(Path::new("gone/kick.wav"))
            .map_err(|err| err.to_string())?
            .is_none()
    );
    Ok(())
}

#[test]
fn recover_pending_retained_restore_relocates_existing_metadata_after_restart() -> Result<(), String>
{
    let (_temp, source) = sample_source();
    let older_existing = source.root.join("older.wav");
    fs::write(&older_existing, b"older-existing").unwrap();
    thread::sleep(Duration::from_millis(20));

    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    fs::write(original.join("kick.wav"), b"newer-staged").unwrap();
    let deleted_entries = vec![sample_entry("gone/kick.wav")];
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = stage_folder_for_delete(
        &original,
        &staging_root,
        Path::new("gone"),
        &deleted_entries,
    )?;
    mark_delete_retained(&staging_root, &staged.id)?;

    fs::create_dir_all(&original).unwrap();
    fs::rename(&older_existing, original.join("kick.wav")).unwrap();
    let db = source.open_db().map_err(|err| err.to_string())?;
    db.upsert_file(Path::new("gone/kick.wav"), 22, 7)
        .map_err(|err| err.to_string())?;
    db.set_tag(
        Path::new("gone/kick.wav"),
        crate::sample_sources::Rating::TRASH_1,
    )
    .map_err(|err| err.to_string())?;
    db.set_last_played_at(Path::new("gone/kick.wav"), 99)
        .map_err(|err| err.to_string())?;

    mark_delete_restore_pending_db(&staging_root, &staged.id, "20260326T105355Z")?;
    restore_retained_folder_with_merge_with_stamp(
        &staged,
        &source.root,
        &source.root.join("gone"),
        &staging_root,
        "20260326T105355Z",
    )?;

    let report = recover_staged_deletes(std::slice::from_ref(&source));
    let db = source.open_db().map_err(|err| err.to_string())?;
    let canonical = db
        .entry_for_path(Path::new("gone/kick.wav"))
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing canonical restored DB row".to_string())?;
    assert_eq!(canonical.tag, deleted_entries[0].tag);
    assert_eq!(canonical.last_played_at, deleted_entries[0].last_played_at);

    let replaced = Path::new("gone/kick.replaced-20260326T105355Z.wav");
    let relocated = db
        .entry_for_path(replaced)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "Missing relocated conflict DB row".to_string())?;
    assert_eq!(relocated.tag, crate::sample_sources::Rating::TRASH_1);
    assert_eq!(relocated.last_played_at, Some(99));
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some("Completed retained restore after restart")
    );
    assert!(report.scan_sources.is_empty());
    Ok(())
}

fn sample_source() -> (tempfile::TempDir, SampleSource) {
    let dir = tempdir().unwrap();
    let root = dir.path().join("source");
    fs::create_dir_all(&root).unwrap();
    (dir, SampleSource::new(root))
}

fn sample_entry(relative_path: &str) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: relative_path.into(),
        file_size: 1024,
        modified_ns: 123,
        content_hash: Some("abc123".into()),
        tag: crate::sample_sources::Rating::KEEP_1,
        looped: true,
        locked: true,
        missing: false,
        last_played_at: Some(456),
    }
}

fn assert_recovery(
    entry: &DeleteRecoveryEntry,
    action: DeleteRecoveryAction,
    status: DeleteRecoveryStatus,
) {
    assert_eq!(entry.action, action);
    assert_eq!(entry.status, status);
}
