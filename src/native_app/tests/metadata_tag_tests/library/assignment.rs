use super::*;

#[test]
fn default_gui_tag_library_can_apply_default_playback_tags() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        toggle_metadata_tag_library(),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("one-shot")),
        &mut ui::UpdateContext::default(),
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
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("bass")),
        &mut ui::UpdateContext::default(),
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
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        toggle_metadata_tag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.ui.status.sample, "Removed tag bass");
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
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(state.metadata.selected_tag.as_deref(), Some("bass"));

    state.apply_message(
        delete_selected_metadata_tag(),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.metadata.selected_tag, None);
    assert_eq!(state.ui.status.sample, "Removed tag bass");
}
