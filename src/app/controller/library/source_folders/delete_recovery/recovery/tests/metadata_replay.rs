use super::*;
use std::thread;
use std::time::Duration;

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
