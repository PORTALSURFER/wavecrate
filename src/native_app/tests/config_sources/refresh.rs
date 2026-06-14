use super::*;

#[test]
fn context_source_refresh_queues_scan_without_clearing_loaded_tree() {
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write sample");
    let mut state = gui_state_for_span_tests();
    let request = state
        .library
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());
    state.ui.browser_interaction.context_menu = Some(
        crate::native_app::test_support::context_menu::BrowserContextMenu {
            kind: crate::native_app::test_support::context_menu::BrowserContextTargetKind::Source,
            path: source_root.path().to_path_buf(),
            source_id: Some(source_id.clone()),
            source_removable: true,
            metadata_tag: None,
            collection: None,
            anchor: Point::new(12.0, 24.0),
            title: String::from("source root"),
        },
    );
    let visible_before = state.library.folder_browser.selected_audio_files().len();
    let mut context = ui::UiUpdateContext::default();

    state.refresh_context_source(&mut context);

    assert_eq!(state.ui.browser_interaction.context_menu, None);
    let task_id = state
        .library
        .folder_progress()
        .expect("refresh should show scan progress")
        .task_id;
    assert!(
        state
            .library
            .folder_browser
            .scan_is_active(&source_id, task_id),
        "refresh should queue the next background scan task"
    );
    assert_eq!(
        state.library.folder_browser.selected_audio_files().len(),
        visible_before,
        "refresh should keep the current cached tree visible while the scan runs"
    );
    assert!(state.ui.status.sample.contains("Scanning source"));
}

#[test]
fn source_filesystem_change_queues_refresh_without_clearing_loaded_tree() {
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums");
    fs::write(drums.join("kick.wav"), [0_u8; 8]).expect("write sample");
    let mut state = gui_state_for_span_tests();
    let request = state
        .library
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 100)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request,
        |_| {},
        |_| {},
    );
    state.finish_folder_scan(result, &mut ui::UiUpdateContext::default());
    let visible_before = state.library.folder_browser.selected_audio_files().len();
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SourceFilesystemChanged {
            source_id: source_id.clone(),
            paths: Vec::new(),
            overflowed: true,
        },
        &mut context,
    );

    let task_id = state
        .library
        .folder_progress()
        .expect("filesystem change should show scan progress")
        .task_id;
    assert!(
        state
            .library
            .folder_browser
            .scan_is_active(&source_id, task_id)
    );
    assert_eq!(
        state.library.folder_browser.selected_audio_files().len(),
        visible_before,
        "live sync should keep the current cached tree visible while the scan runs"
    );
}
