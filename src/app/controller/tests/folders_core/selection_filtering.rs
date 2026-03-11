use super::support::*;

#[test]
fn selecting_root_filters_to_root_files() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("rooted");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&source.root.join("root.wav"), &[0.2, -0.2]);
    write_test_wav(&folder.join("clip.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("root.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("rooted/clip.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let folder_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("rooted"))
        .unwrap();

    controller.replace_folder_selection(folder_index);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("rooted/clip.wav")]
    );

    controller.replace_folder_selection(0);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("root.wav"), PathBuf::from("rooted/clip.wav")]
    );
    assert_eq!(controller.ui.sources.folders.focused, Some(0));

    controller.replace_folder_selection(0);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("root.wav")]
    );

    controller.toggle_folder_row_selection(folder_index);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("root.wav"), PathBuf::from("rooted/clip.wav")]
    );
    Ok(())
}

#[test]
fn clearing_folder_selection_shows_all_samples() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    std::fs::create_dir_all(source.root.join("b")).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a/one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
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

    assert_eq!(controller.selected_folder_paths(), vec![PathBuf::from("a")]);
    assert_eq!(visible_indices(&controller), vec![0]);

    controller.clear_folder_selection();

    assert!(controller.selected_folder_paths().is_empty());
    assert_eq!(visible_indices(&controller), vec![0, 1]);
    Ok(())
}

#[test]
fn negated_folder_hides_samples() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("a")).unwrap();
    std::fs::create_dir_all(source.root.join("b")).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a/one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b/two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
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
    controller.toggle_folder_row_negation(folder_a);

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("b/two.wav")]
    );
    Ok(())
}

#[test]
fn negated_root_hides_only_root_samples() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("sub")).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("root.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("sub/child.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.toggle_folder_row_negation(0);

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("sub/child.wav")]
    );
    Ok(())
}
