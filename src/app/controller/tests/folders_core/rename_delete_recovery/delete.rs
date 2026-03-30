use super::super::support::*;

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
