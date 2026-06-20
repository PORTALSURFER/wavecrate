use super::*;

#[test]
fn default_gui_tag_library_can_apply_default_playback_tags() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        toggle_metadata_tag_library(),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("one-shot")),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("one-shot")])
    );
}

#[test]
fn default_gui_tag_library_button_adds_existing_tag() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );

    state.apply_message(
        toggle_metadata_tag_library(),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("bass")),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("bass")])
    );
}

#[test]
fn default_gui_tag_library_button_removes_selected_tag() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("bass"), String::from("hat")],
    );
    state
        .metadata
        .tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);

    state.apply_message(
        toggle_metadata_tag_library(),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("bass")),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.ui.status.sample, "Removed tag bass");
}

#[test]
fn mixed_metadata_tag_toggle_adds_missing_tag_to_all_selected_samples() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("first.wav");
    let second = source_root.path().join("second.wav");
    fs::write(&first, []).expect("first sample");
    fs::write(&second, []).expect("second sample");
    let source = wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf());
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    state.library.folder_browser.select_file(first_id.clone());
    state.library.folder_browser.select_file_with_modifiers(
        second_id.clone(),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    state.metadata.tags_by_file.insert(
        first_id.clone(),
        vec![String::from("bass"), String::from("hat")],
    );
    state
        .metadata
        .tags_by_file
        .insert(second_id.clone(), vec![String::from("dry")]);

    state.apply_message(
        toggle_metadata_tag(String::from("bass")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&first_id),
        Some(&vec![String::from("bass"), String::from("hat")])
    );
    assert_eq!(
        state.metadata.tags_by_file.get(&second_id),
        Some(&vec![String::from("dry"), String::from("bass")])
    );
    assert!(state.metadata_tag_selection_state("bass").is_all());
}

#[test]
fn metadata_tag_chip_selection_can_be_deleted_from_selected_sample() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("bass"), String::from("hat")],
    );

    state.apply_message(
        select_metadata_tag(String::from("bass")),
        &mut ui::UiUpdateContext::default(),
    );
    assert_eq!(state.metadata.selected_tag.as_deref(), Some("bass"));

    state.apply_message(
        delete_selected_metadata_tag(),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.metadata.selected_tag, None);
    assert_eq!(state.ui.status.sample, "Removed tag bass");
}

#[test]
fn metadata_tag_chip_deletion_removes_tag_from_all_selected_samples() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("first.wav");
    let second = source_root.path().join("second.wav");
    fs::write(&first, []).expect("first sample");
    fs::write(&second, []).expect("second sample");
    let source = wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf());
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    state.library.folder_browser.select_file(first_id.clone());
    state.library.folder_browser.select_file_with_modifiers(
        second_id.clone(),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    state.metadata.tags_by_file.insert(
        first_id.clone(),
        vec![String::from("bass"), String::from("hat")],
    );
    state.metadata.tags_by_file.insert(
        second_id.clone(),
        vec![String::from("bass"), String::from("dry")],
    );

    state.apply_message(
        select_metadata_tag(String::from("bass")),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        delete_selected_metadata_tag(),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&first_id),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(
        state.metadata.tags_by_file.get(&second_id),
        Some(&vec![String::from("dry")])
    );
    assert_eq!(state.metadata.selected_tag, None);
    assert_eq!(state.ui.status.sample, "Removed tag bass from 2 samples");
}
