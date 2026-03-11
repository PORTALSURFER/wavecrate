use super::support::*;

#[test]
fn creating_folder_tracks_manual_entry() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.refresh_folder_browser_for_tests();
    assert!(controller.ui.sources.folders.rows[0].is_root);

    controller.create_folder(Path::new(""), "NewFolder")?;

    assert!(source.root.join("NewFolder").is_dir());
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("NewFolder"))
    );
    Ok(())
}

#[test]
fn folder_browser_includes_root_entry() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.refresh_folder_browser_for_tests();

    let rows = &controller.ui.sources.folders.rows;
    assert!(
        rows.first()
            .is_some_and(|row| row.is_root && row.path.as_os_str().is_empty())
    );
}

#[test]
fn folder_browser_lists_empty_folders() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let empty = source.root.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    controller.refresh_folder_browser_for_tests();

    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("empty"))
    );
}

#[test]
fn root_entry_stays_above_real_folders() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("rooted");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "rooted/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let rows = &controller.ui.sources.folders.rows;
    assert!(rows.first().is_some_and(|row| row.is_root));
    assert!(
        rows.get(1)
            .is_some_and(|row| row.path == PathBuf::from("rooted"))
    );
}

#[test]
fn start_new_folder_at_root_sets_root_parent() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.refresh_folder_browser_for_tests();

    controller.start_new_folder_at_root();

    let new_folder = controller.ui.sources.folders.new_folder.as_ref().unwrap();
    assert!(new_folder.parent.as_os_str().is_empty());
}

#[test]
fn start_new_folder_uses_focused_parent() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("clips");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "clips/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let folder_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("clips"))
        .unwrap();

    controller.focus_folder_row(folder_index);
    controller.start_new_folder();

    let new_folder = controller.ui.sources.folders.new_folder.as_ref().unwrap();
    assert_eq!(new_folder.parent, PathBuf::from("clips"));
    assert!(new_folder.focus_requested);
}

#[test]
fn start_new_folder_clears_search_query() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.refresh_folder_browser_for_tests();
    controller.set_folder_search("kick".to_string());
    assert_eq!(controller.ui.sources.folders.search_query, "kick");

    controller.start_new_folder();

    assert!(controller.ui.sources.folders.search_query.is_empty());
    assert!(controller.ui.sources.folders.new_folder.is_some());
}

#[test]
fn cancelling_new_folder_creation_clears_state() {
    let (mut controller, _) = dummy_controller();
    controller.ui.sources.folders.new_folder = Some(crate::app::state::InlineFolderCreation {
        parent: PathBuf::new(),
        name: "temp".into(),
        focus_requested: false,
    });

    controller.cancel_new_folder_creation();

    assert!(controller.ui.sources.folders.new_folder.is_none());
}
