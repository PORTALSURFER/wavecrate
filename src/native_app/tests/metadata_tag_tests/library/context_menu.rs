use super::*;

#[test]
fn default_gui_tag_library_right_click_opens_tag_context_menu() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(String::from("other.wav"), vec![String::from("oneshot")]);
    state
        .metadata
        .tag_dictionary
        .insert(String::from("oneshot"), String::from("sound-type"));
    state.metadata.tag_library_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let tag_rect = frame
        .paint_plan
        .first_text_rect("oneshot")
        .expect("oneshot tag should paint");
    let point = tag_rect.center();

    runtime.dispatch_event(Event::secondary_press(point));

    let menu = runtime
        .bridge()
        .state()
        .ui
        .browser_interaction
        .context_menu
        .as_ref()
        .expect("right-click should open metadata tag context menu");
    assert_eq!(
        menu.kind,
        crate::native_app::test_support::context_menu::BrowserContextTargetKind::MetadataTag
    );
    assert_eq!(menu.metadata_tag.as_deref(), Some("oneshot"));
}

#[test]
fn metadata_tag_context_delete_removes_unlocked_global_tag() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("oneshot"), String::from("hat")],
    );
    state.metadata.tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("oneshot"), String::from("seq")],
    );
    state
        .metadata
        .tag_dictionary
        .insert(String::from("oneshot"), String::from("sound-type"));
    state.ui.browser_interaction.context_menu = Some(
        crate::native_app::test_support::context_menu::BrowserContextMenu {
            kind:
                crate::native_app::test_support::context_menu::BrowserContextTargetKind::MetadataTag,
            path: PathBuf::new(),
            source_id: None,
            source_removable: false,
            metadata_tag: Some(String::from("oneshot")),
            collection: None,
            anchor: Point::new(12.0, 24.0),
            title: String::from("oneshot"),
        },
    );

    state.delete_context_metadata_tag(&mut ui::UiUpdateContext::default());

    assert!(!state.metadata.tag_dictionary.contains_key("oneshot"));
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(
        state.metadata.tags_by_file.get("other.wav"),
        Some(&vec![String::from("seq")])
    );
    assert_eq!(state.ui.browser_interaction.context_menu, None);
    assert_eq!(
        state.ui.status.sample,
        "Deleted tag oneshot from 2 assignment(s)"
    );
}

#[test]
fn metadata_tag_context_delete_rejects_locked_playback_tags() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.ui.browser_interaction.context_menu = Some(
        crate::native_app::test_support::context_menu::BrowserContextMenu {
            kind:
                crate::native_app::test_support::context_menu::BrowserContextTargetKind::MetadataTag,
            path: PathBuf::new(),
            source_id: None,
            source_removable: false,
            metadata_tag: Some(String::from("loop")),
            collection: None,
            anchor: Point::new(12.0, 24.0),
            title: String::from("loop"),
        },
    );

    state.delete_context_metadata_tag(&mut ui::UiUpdateContext::default());

    assert_eq!(state.ui.status.sample, "Playback Type tags are locked");
}
