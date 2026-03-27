use super::support::*;

#[test]
fn folder_browser_defaults_to_showing_empty_disk_folders() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let sampled = source.root.join("drums").join("kicks");
    let empty = source.root.join("drums").join("empty");
    std::fs::create_dir_all(&sampled).unwrap();
    std::fs::create_dir_all(&empty).unwrap();
    write_test_wav(&sampled.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.refresh_folder_browser_for_tests();

    assert!(controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/empty"))
    );
    Ok(())
}

#[test]
fn toggling_folder_visibility_hides_and_restores_empty_folders() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let sampled = source.root.join("drums").join("kicks");
    let empty = source.root.join("drums").join("empty");
    std::fs::create_dir_all(&sampled).unwrap();
    std::fs::create_dir_all(&empty).unwrap();
    write_test_wav(&sampled.join("tight.wav"), &[0.2, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "drums/kicks/tight.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.toggle_show_all_folders();

    assert!(!controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != PathBuf::from("drums/empty"))
    );
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/kicks"))
    );

    controller.toggle_show_all_folders();

    assert!(controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("drums/empty"))
    );
    Ok(())
}
