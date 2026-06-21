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
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection: None,
            sample_missing: false,
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

#[test]
fn source_filesystem_change_syncs_removed_file_to_source_database() {
    let source_root = tempfile::tempdir().expect("source root");
    fs::write(source_root.path().join("stale.wav"), [0_u8; 8]).expect("write stale sample");
    fs::write(source_root.path().join("keep.wav"), [1_u8; 8]).expect("write keep sample");
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
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path()).expect("db");
    assert_eq!(db.list_files().expect("seeded rows").len(), 2);
    fs::remove_file(source_root.path().join("stale.wav")).expect("remove stale sample");
    let mut context = ui::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SourceFilesystemChanged {
            source_id: source_id.clone(),
            paths: vec![PathBuf::from("stale.wav")],
            overflowed: false,
        },
        &mut context,
    );

    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>(),
        vec!["keep.wav"],
        "bounded filesystem patch should remove deleted sample from the visible list immediately"
    );
    let sync_finished =
        run_named_perform(context.into_command(), "gui-source-db-sync").expect("db sync command");
    state.apply_message(sync_finished, &mut ui::UiUpdateContext::default());

    let rows = db.list_files().expect("synced rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, std::path::Path::new("keep.wav"));
}

fn run_named_perform(
    command: Command<crate::native_app::test_support::state::GuiMessage>,
    target_name: &'static str,
) -> Option<crate::native_app::test_support::state::GuiMessage> {
    match command {
        Command::Perform { name, work, .. } if name == target_name => Some(work()),
        Command::Batch(commands) => commands
            .into_iter()
            .find_map(|command| run_named_perform(command, target_name)),
        _ => None,
    }
}
