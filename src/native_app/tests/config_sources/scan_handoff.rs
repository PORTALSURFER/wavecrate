use super::*;

#[test]
fn adding_source_after_startup_registers_it_before_scan_admission_and_finish() {
    let source_root = tempfile::tempdir().expect("source root");
    let mut state = gui_state_for_span_tests();
    let request = state
        .library
        .folder_browser
        .begin_add_source_path(source_root.path().to_path_buf(), 101)
        .expect("new source requests scan");
    let source_id = request.source_id.clone();
    let request_for_worker = request.clone();
    let mut context = ui::UiUpdateContext::default();

    state.launch_folder_scan(request, &mut context);

    let permit = state
        .background
        .source_processing
        .budget_handle()
        .acquire_scan(&source_id)
        .expect("newly added source must be admitted before its first scan");
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        request_for_worker,
        |_| {},
        |_| {},
    );
    drop(permit);
    state.finish_folder_scan(result, &mut context);

    assert!(state.library.folder_browser.selected_source_loaded());
    assert!(state.library.folder_progress().is_none());
}

#[test]
fn source_filesystem_change_during_scan_is_refreshed_after_scan_finishes() {
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
    let mut context = ui::UiUpdateContext::default();
    state.refresh_source_after_filesystem_change(source_id.clone(), Vec::new(), true, &mut context);

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SourceFilesystemChanged {
            source_id: source_id.clone(),
            paths: Vec::new(),
            overflowed: true,
        },
        &mut context,
    );
    assert!(
        state
            .library
            .pending_source_refresh_contains_for_tests(&source_id)
    );

    let active_task = state
        .library
        .folder_progress()
        .expect("first refresh should be active")
        .task_id;
    assert!(
        state
            .library
            .folder_browser
            .scan_is_active(&source_id, active_task),
        "first scan should still own the active task"
    );
    let finished =
        crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
            crate::native_app::sample_library::folder_browser::scan::FolderScanRequest {
                task_id: active_task,
                source_id: source_id.clone(),
                label: String::from("source"),
                root: source_root.path().to_path_buf(),
                database_root: source_root.path().to_path_buf(),
                rating_decay_weeks: crate::native_app::sample_library::folder_browser::scan::FolderScanRequest::default_rating_decay_weeks(),
            },
            |_| {},
            |_| {},
        );
    state.finish_folder_scan(finished, &mut ui::UiUpdateContext::default());
    state.maybe_run_pending_source_refresh(&mut context);

    let next_task = state
        .library
        .folder_progress()
        .expect("pending refresh should start after active scan")
        .task_id;
    assert_ne!(next_task, active_task);
    assert!(
        state
            .library
            .folder_browser
            .scan_is_active(&source_id, next_task)
    );
}
