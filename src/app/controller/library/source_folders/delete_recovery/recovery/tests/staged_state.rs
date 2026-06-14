use super::*;

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
