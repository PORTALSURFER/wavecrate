use super::*;

#[test]
fn metadata_tag_input_submits_typed_prefix_without_autoselecting_completion() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata_tag_completion_suffix().as_deref(),
        Some("ck")
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("ki"));
    assert_eq!(state.ui.status.sample, "Choose a category for ki");
}

#[test]
fn metadata_tag_completion_request_shows_suggestions_without_selecting_one() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata.tags_by_file.get(&selected_file), None);
    assert_eq!(state.pending_metadata_tag_category_tag(), Some("ki"));
    assert_eq!(state.ui.status.sample, "Choose a category for ki");
}

#[test]
fn metadata_tag_second_completion_request_activates_first_suggestion() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![String::from("kick"), String::from("warm")],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::CompletionRequested {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kick")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("kick")])
    );
}

#[test]
fn metadata_tag_input_arrows_through_multiple_known_prefix_matches() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        String::from("known-file"),
        vec![
            String::from("kick"),
            String::from("kicker"),
            String::from("kind"),
        ],
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        None
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kick")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kicker")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MoveMetadataTagCompletion(1),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state
            .metadata_tag_completion_options()
            .iter()
            .find(|option| option.selected)
            .map(|option| option.tag.as_str()),
        Some("kind")
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("ki"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("kind")])
    );
    assert!(state.metadata.tag_draft.is_empty());
}
