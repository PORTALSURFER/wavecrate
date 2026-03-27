use super::support::*;

fn folder_row_index(controller: &AppController, path: &str) -> usize {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from(path))
        .unwrap_or_else(|| panic!("missing folder row for {path}"))
}

#[test]
fn folder_tree_uses_root_label_and_child_depths() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let root = controller
        .ui
        .sources
        .folders
        .rows
        .first()
        .expect("root row should be present");
    assert_eq!(root.name, "Root");
    assert_eq!(root.depth, 0);

    let drums = &controller.ui.sources.folders.rows[folder_row_index(&controller, "drums")];
    let kicks = &controller.ui.sources.folders.rows[folder_row_index(&controller, "drums/kicks")];
    assert_eq!(drums.depth, 1);
    assert_eq!(kicks.depth, 2);
    Ok(())
}

#[test]
fn toggling_folder_expansion_hides_and_restores_descendants() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let drums = folder_row_index(&controller, "drums");
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    controller.toggle_folder_expanded(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/kicks"))
    );

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );
    Ok(())
}

#[test]
fn collapsing_leaf_folder_focuses_parent_row() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let kicks = folder_row_index(&controller, "drums/kicks");
    controller.replace_folder_selection(kicks);
    controller.collapse_focused_folder();

    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}

#[test]
fn nudge_folder_focus_moves_through_visible_rows() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after first move");
    assert_eq!(controller.ui.sources.folders.rows[focused].path, PathBuf::from(""));

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after second move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after third move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums/kicks")
    );

    controller.nudge_folder_focus_action(-1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after moving back up");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}

#[test]
fn nudge_folder_focus_skips_collapsed_descendants() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    controller.nudge_folder_focus_action(1);
    controller.nudge_folder_focus_action(1);

    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after moving through collapsed tree");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );

    controller.nudge_folder_focus_action(1);
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused folder row after clamped move");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("drums")
    );
    Ok(())
}

#[test]
fn activating_expandable_folder_row_expands_then_collapses_it() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/kicks"))
    );
    Ok(())
}

#[test]
fn reactivating_folder_after_focusing_elsewhere_keeps_it_expanded() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let drums = folder_row_index(&controller, "drums");
    controller.toggle_folder_expanded(drums);
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    controller.ui.focus.context = FocusContext::Waveform;

    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/kicks"))
    );
    Ok(())
}

#[test]
fn activating_leaf_folder_row_keeps_tree_projection_unchanged() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    let before: Vec<PathBuf> = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect();
    let kicks = folder_row_index(&controller, "drums/kicks");
    controller.activate_folder_row(kicks);

    let after: Vec<PathBuf> = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect();
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn activating_folder_search_result_does_not_toggle_expansion() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.set_folder_search(String::from("drum"));
    let drums = folder_row_index(&controller, "drums");
    controller.activate_folder_row(drums);
    controller.set_folder_search(String::new());

    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );
    Ok(())
}
