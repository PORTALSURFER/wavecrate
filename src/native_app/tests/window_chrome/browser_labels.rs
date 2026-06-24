use super::*;

#[test]
fn default_folder_browser_loads_assets_root() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    assert!(browser.root_path().ends_with("assets"));
    assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
    assert!(
        !browser.selected_files().is_empty(),
        "default assets source should expose bundled asset files"
    );
    assert!(
        browser.selected_audio_files().is_empty(),
        "bundled font/icon assets should not appear as audio samples"
    );
}

#[test]
fn sample_browser_toggles_between_disk_and_metadata_label_names() {
    let (mut state, _source_root, tagged_file) =
        native_app_state_with_temp_sample("tag-toggle.wav");
    state.metadata.tags_by_file.insert(
        tagged_file,
        vec![String::from("kick"), String::from("warm")],
    );
    prepare_sample_browser_view(&mut state);
    let disk_frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));
    assert!(disk_frame.paint_plan.contains_text("Disk"));

    state.apply_message(
        toggle_sample_name_view_mode(),
        &mut ui::UiUpdateContext::default(),
    );
    prepare_sample_browser_view(&mut state);
    let label_frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));

    assert!(label_frame.paint_plan.contains_text("Label"));
}

#[test]
fn sample_browser_random_navigation_toggle_renders_and_updates_state() {
    let (mut state, _source_root, _file) = native_app_state_with_temp_sample("random-toggle.wav");
    prepare_sample_browser_view(&mut state);
    let disk_frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));
    let dice_rect = disk_frame
        .layout
        .rects
        .get(&crate::native_app::ui::ids::AUTOMATION_SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID)
        .expect("random navigation toggle should be laid out");
    let disk_label = disk_frame
        .paint_plan
        .text_runs()
        .find(|text| text.text == "Disk")
        .expect("disk/label toggle should paint");
    assert!(
        dice_rect.max.x <= disk_label.rect.min.x,
        "dice toggle should sit to the left of the disk/label toggle"
    );
    assert!(!state.library.folder_browser.random_navigation_enabled());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::ToggleRandomNavigationMode,
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.library.folder_browser.random_navigation_enabled());
    assert_eq!(state.ui.status.sample, "Random sample navigation on");
}
