use super::*;

#[test]
fn default_folder_browser_loads_assets_root() {
    let browser = crate::native_app::test_support::FolderBrowserState::load_default();
    assert!(browser.root_path().ends_with("assets"));
    assert_eq!(browser.source_labels(), vec![String::from("Assets")]);
    assert!(
        browser
            .selected_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_001.wav")
    );
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .any(|file| file.name == "portal_SS_kick_001.wav")
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
    let disk_frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));
    assert!(disk_frame.paint_plan.contains_text("Disk"));

    state.apply_message(
        toggle_sample_name_view_mode(),
        &mut ui::UpdateContext::default(),
    );
    let label_frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 240.0));

    assert!(label_frame.paint_plan.contains_text("Label"));
}
