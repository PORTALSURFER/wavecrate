use super::*;

#[test]
fn metadata_tag_input_prompts_for_category_before_adding_new_tag() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert_eq!(
        state.metadata_tag_input_placeholder(),
        "select group/parent tag"
    );
    assert_eq!(state.ui.status.sample, "Choose a category for deep-kick");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("sound"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Sound Type")
    );

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("sound"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(state.metadata.tag_draft.is_empty());
    assert_eq!(state.ui.status.sample, "Added tag deep-kick");
}

#[test]
fn metadata_tag_category_selection_shows_all_options_immediately() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    let options = state.metadata_tag_completion_options();
    assert_eq!(
        options
            .iter()
            .map(|option| (option.tag.as_str(), option.selected))
            .collect::<Vec<_>>(),
        vec![
            ("Sound Type", true),
            ("Character", false),
            ("Prefix", false),
            ("Tuning/Scale", false),
        ]
    );
    assert!(state.metadata_tag_completion_active());

    state.apply_message(
        move_metadata_tag_completion(1),
        &mut ui::UiUpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Character")
    );

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::new(),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("character")
    );
}

#[test]
fn metadata_tag_category_hover_updates_active_selection() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        hover_metadata_tag_completion(String::from("Character")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Character")
    );

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::new(),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("character")
    );
}

#[test]
fn metadata_tag_category_keyboard_navigation_continues_from_hover_selection() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        hover_metadata_tag_completion(String::from("Character")),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        move_metadata_tag_completion(1),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Prefix")
    );
}

#[test]
/// Hovering a category completion row by category id should select the same row as its label.
fn metadata_tag_category_hover_accepts_category_id() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        hover_metadata_tag_completion(String::from("prefix")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("Prefix")
    );
}

#[test]
fn metadata_tag_category_cancel_aborts_pending_tag_entry() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("sound"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert!(state.metadata_tag_completion_active());

    state.apply_message(
        cancel_metadata_tag_entry(),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(!state.metadata_tag_completion_active());
    assert_eq!(state.metadata_tag_input_placeholder(), "add tag");
    assert!(state.metadata.tag_draft.is_empty());
    assert!(state.metadata.tag_tokens.is_empty());
    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.metadata.tag_dictionary.get("deep-kick"), None);
}

#[test]
fn metadata_tag_category_invalid_completion_selection_keeps_enter_commit_available() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("Deep Kick"),
        }),
        &mut ui::UiUpdateContext::default(),
    );
    state.apply_message(
        select_metadata_tag_completion(String::from("Not a category")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.pending_metadata_tag_category_tag(), Some("deep-kick"));
    assert!(state.metadata_tag_completion_active());
    assert_eq!(state.ui.status.sample, "Choose a category for deep-kick");

    state.apply_message(
        metadata_tag_input(radiant::widgets::TextInputMessage::Submitted {
            value: String::from("sound"),
        }),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick")])
    );
    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("deep-kick")
            .map(String::as_str),
        Some("sound-type")
    );
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
}
