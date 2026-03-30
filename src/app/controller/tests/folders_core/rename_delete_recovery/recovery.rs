use super::super::support::*;

#[test]
fn staged_delete_recovery_restores_after_crash() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("gone");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&target.join("sample.wav"), &[0.0, 0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "gone/sample.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    if let Some(index) = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("gone"))
    {
        controller.focus_folder_row(index);
    }
    controller.runtime.fail_after_folder_delete_stage = true;

    controller.delete_focused_folder();

    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
    assert!(staging_root.exists());
    assert!(!target.exists());

    let report = delete_recovery::recover_staged_deletes(std::slice::from_ref(&source));

    assert!(target.exists());
    assert!(!staging_root.exists());
    assert!(report.entries.iter().any(|entry| {
        entry.action == delete_recovery::DeleteRecoveryAction::Restore
            && entry.status == delete_recovery::DeleteRecoveryStatus::Completed
    }));
    Ok(())
}

#[test]
fn staged_delete_recovery_retains_deleted_folder_after_db_commit_crash() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("gone");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&target.join("sample.wav"), &[0.0, 0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "gone/sample.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    if let Some(index) = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("gone"))
    {
        controller.focus_folder_row(index);
    }
    controller.runtime.fail_after_folder_delete_db_commit = true;

    controller.delete_focused_folder();

    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
    assert!(staging_root.exists());
    assert!(!target.exists());

    let report = delete_recovery::recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!target.exists());
    assert!(staging_root.exists());
    assert!(report.entries.is_empty());
    assert_eq!(report.retained_entries.len(), 1);
    let retained = &report.retained_entries[0];
    assert_eq!(retained.original_relative, PathBuf::from("gone"));
    assert_eq!(retained.deleted_entries.len(), 1);
    assert_eq!(
        retained.deleted_entries[0].relative_path,
        PathBuf::from("gone/sample.wav")
    );
    Ok(())
}
