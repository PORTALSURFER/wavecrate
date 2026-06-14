use super::*;

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
