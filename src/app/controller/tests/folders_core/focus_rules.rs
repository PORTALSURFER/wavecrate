use super::support::*;

#[test]
fn folder_focus_clears_when_context_changes() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("one");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("sample.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one/sample.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let row_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("one"))
        .unwrap();

    controller.replace_folder_selection(row_index);
    assert_eq!(controller.ui.sources.folders.focused, Some(row_index));

    controller.focus_browser_context();

    assert!(controller.ui.sources.folders.focused.is_none());
    controller.refresh_folder_browser_for_tests();
    assert!(controller.ui.sources.folders.focused.is_none());
    assert_eq!(
        controller.selected_folder_paths(),
        vec![PathBuf::from("one")]
    );
    Ok(())
}

#[test]
fn escape_does_not_clear_folder_filter_without_folder_focus() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "a/one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let folder_a = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("a"))
        .unwrap();
    controller.replace_folder_selection(folder_a);
    controller.ui.focus.context = FocusContext::SampleBrowser;

    controller.handle_escape();

    assert_eq!(controller.selected_folder_paths(), vec![PathBuf::from("a")]);
    Ok(())
}
