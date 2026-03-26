use super::support::*;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionMode,
};

#[test]
fn renaming_folder_updates_entries_and_tree() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.rename_folder(Path::new("old"), "new")?;

    assert!(!folder.exists());
    assert!(source.root.join("new/clip.wav").is_file());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("new/clip.wav")
    );
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("new"))
    );
    Ok(())
}

#[test]
fn cancelling_folder_rename_clears_prompt() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.sources.folders.pending_action =
        Some(crate::app::state::FolderActionPrompt::Rename {
            target: PathBuf::from("folder"),
            name: "folder".into(),
        });
    controller.ui.sources.folders.rename_focus_requested = true;

    controller.cancel_folder_rename();

    assert!(controller.ui.sources.folders.pending_action.is_none());
    assert!(!controller.ui.sources.folders.rename_focus_requested);
}

#[test]
fn deleting_folder_removes_wavs() -> Result<(), String> {
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

    controller.delete_focused_folder();

    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
    assert!(!target.exists());
    assert!(staging_root.exists());
    assert_eq!(controller.wav_entries_len(), 0);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("gone"))
    );
    Ok(())
}

#[test]
fn deleting_folder_supports_undo_and_redo() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("gone");
    let sample = target.join("sample.wav");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&sample, &[0.0, 0.2]);
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

    controller.delete_focused_folder();
    assert!(!target.exists());

    controller.undo();
    assert!(target.exists());
    assert!(sample.is_file());
    assert_eq!(controller.wav_entries_len(), 1);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("gone"))
    );

    controller.redo();
    assert!(!target.exists());
    assert_eq!(controller.wav_entries_len(), 0);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("gone"))
    );
    Ok(())
}

#[test]
fn deleting_folder_rolls_back_on_db_failure() -> Result<(), String> {
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
    controller.runtime.fail_next_folder_delete_db = true;

    controller.delete_focused_folder();

    assert!(target.exists());
    assert_eq!(controller.wav_entries_len(), 1);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("gone"))
    );
    let db = crate::sample_sources::SourceDatabase::open(&source.root).unwrap();
    assert_eq!(db.count_files().unwrap(), 1);
    Ok(())
}

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

#[test]
fn deleting_folder_moves_focus_to_next_available() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    for folder in ["a", "b", "c"] {
        let path = source.root.join(folder);
        std::fs::create_dir_all(&path).unwrap();
        write_test_wav(&path.join(format!("{folder}.wav")), &[0.0, 0.2]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a/a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/b.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("c/c.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let focus_row = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("b"))
        .unwrap();
    controller.focus_folder_row(focus_row);

    controller.delete_focused_folder();

    let focused = controller.ui.sources.folders.focused.unwrap();
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("c")
    );

    controller.delete_focused_folder();

    let focused = controller.ui.sources.folders.focused.unwrap();
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("a")
    );
    Ok(())
}

#[test]
fn deleting_folder_warns_when_retained_recovery_is_processing_the_folder() -> Result<(), String> {
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
    let focus_row = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("gone"))
        .unwrap();
    controller.focus_folder_row(focus_row);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("gone"),
        }],
    });

    controller.delete_focused_folder();

    assert!(target.exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
    Ok(())
}

#[test]
fn confirming_folder_rename_warns_when_retained_recovery_is_processing_the_folder() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let target = source.root.join("old");
    std::fs::create_dir_all(&target).unwrap();
    write_test_wav(&target.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    controller.ui.sources.folders.pending_action =
        Some(crate::app::state::FolderActionPrompt::Rename {
            target: PathBuf::from("old"),
            name: "new".into(),
        });
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("old"),
        }],
    });

    assert!(controller.confirm_active_prompt());

    assert!(controller.ui.sources.folders.pending_action.is_some());
    assert!(target.exists());
    assert!(!source.root.join("new").exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}
