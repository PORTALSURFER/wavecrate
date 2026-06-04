use super::super::*;

#[test]
fn default_gui_tag_library_opens_beside_folder_sidebar() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("hat"), String::from("seq")],
    );
    state.metadata_tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );
    state.metadata_tag_library_open = true;

    let frame = super::super::super::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("Tag Editor"));
    assert!(frame.paint_plan.contains_text("Playback Type (2) [locked]"));
    assert!(frame.paint_plan.contains_text("Sound Type (2)"));
    assert!(frame.paint_plan.contains_text("Character (1)"));
    assert!(frame.paint_plan.contains_text("Prefix"));
    assert!(frame.paint_plan.contains_text("Tuning/Scale"));
    assert!(frame.paint_plan.contains_text("loop"));
    assert!(frame.paint_plan.contains_text("one-shot"));
    assert!(frame.paint_plan.contains_text("hat"));
    assert!(frame.paint_plan.contains_text("bass"));
    assert!(frame.paint_plan.contains_text("seq"));
}

#[test]
fn default_gui_tag_library_can_apply_default_playback_tags() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");

    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTagLibrary,
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTag(String::from("one-shot")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tag_library_open);
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("one-shot")])
    );
}

#[test]
fn default_gui_tag_library_button_adds_existing_tag() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );

    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTagLibrary,
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tag_library_open);
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("bass")])
    );
}

#[test]
fn default_gui_tag_library_button_removes_selected_tag() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("bass"), String::from("hat")],
    );
    state
        .metadata_tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);

    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTagLibrary,
        &mut ui::UpdateContext::default(),
    );
    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata_tag_library_open);
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.sample_status, "Removed tag bass");
}

#[test]
fn metadata_tag_chip_selection_can_be_deleted_from_selected_sample() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("bass"), String::from("hat")],
    );

    state.apply_message(
        super::super::super::GuiMessage::SelectMetadataTag(String::from("bass")),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(state.selected_metadata_tag.as_deref(), Some("bass"));

    state.apply_message(
        super::super::super::GuiMessage::DeleteSelectedMetadataTag,
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(state.selected_metadata_tag, None);
    assert_eq!(state.sample_status, "Removed tag bass");
}

#[test]
fn default_gui_tag_library_category_headers_collapse_groups() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    state.metadata_tag_library_open = true;

    state.apply_message(
        super::super::super::GuiMessage::ToggleMetadataTagCategory(String::from("sound-type")),
        &mut ui::UpdateContext::default(),
    );

    let frame = super::super::super::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("Sound Type (1)"));
    assert!(
        !frame
            .paint_plan
            .contains_text_after_x("hat", DEFAULT_FOLDER_WIDTH)
    );
}

#[test]
fn default_gui_tag_library_drag_moves_tag_between_categories() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);

    state.drag_metadata_tag(
        String::from("bass"),
        DragHandleMessage::started(Point::new(10.0, 10.0)),
        &mut ui::UpdateContext::default(),
    );
    state.drop_metadata_tag_on_category(
        String::from("character"),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata_tag_dictionary
            .get("bass")
            .map(String::as_str),
        Some("character")
    );
    assert_eq!(state.sample_status, "Moved tag bass to Character");
}

#[test]
fn default_gui_tag_library_rejects_dragging_locked_playback_tags() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");

    state.drag_metadata_tag(
        String::from("one-shot"),
        DragHandleMessage::started(Point::new(10.0, 10.0)),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(state.metadata_tag_drag, None);
    assert_eq!(state.metadata_tag_dictionary.get("one-shot"), None);
    assert_eq!(state.sample_status, "Playback Type tags are locked");
}

#[test]
fn default_gui_tag_library_pointer_drag_drops_tag_on_category_header() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);
    state.metadata_tag_library_open = true;
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let bass_rect = frame
        .paint_plan
        .first_text_rect("bass")
        .expect("bass tag should paint");
    let character_rect = frame
        .paint_plan
        .first_text_rect("Character")
        .expect("character header should paint");
    let bass_point = bass_rect.center();
    let character_point = character_rect.center();

    runtime.dispatch_event(Event::primary_press(bass_point));
    runtime.dispatch_event(Event::pointer_move(Point::new(
        bass_point.x + 8.0,
        bass_point.y + 2.0,
    )));
    runtime.dispatch_event(Event::primary_release(character_point));

    assert_eq!(
        runtime
            .bridge()
            .state()
            .metadata_tag_dictionary
            .get("bass")
            .map(String::as_str),
        Some("character")
    );
}

#[test]
fn default_gui_tag_library_right_click_opens_tag_context_menu() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(String::from("other.wav"), vec![String::from("oneshot")]);
    state
        .metadata_tag_dictionary
        .insert(String::from("oneshot"), String::from("sound-type"));
    state.metadata_tag_library_open = true;
    let mut runtime = gui_runtime_for_tests(state, Vector2::new(900.0, 620.0));
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
        .context_menu
        .as_ref()
        .expect("right-click should open metadata tag context menu");
    assert_eq!(
        menu.kind,
        super::super::super::BrowserContextTargetKind::MetadataTag
    );
    assert_eq!(menu.metadata_tag.as_deref(), Some("oneshot"));
}

#[test]
fn metadata_tag_context_delete_removes_unlocked_global_tag() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("oneshot"), String::from("hat")],
    );
    state.metadata_tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("oneshot"), String::from("seq")],
    );
    state
        .metadata_tag_dictionary
        .insert(String::from("oneshot"), String::from("sound-type"));
    state.context_menu = Some(super::super::super::BrowserContextMenu {
        kind: super::super::super::BrowserContextTargetKind::MetadataTag,
        path: PathBuf::new(),
        source_id: None,
        metadata_tag: Some(String::from("oneshot")),
        collection: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("oneshot"),
    });

    state.delete_context_metadata_tag(&mut ui::UpdateContext::default());

    assert!(!state.metadata_tag_dictionary.contains_key("oneshot"));
    assert_eq!(
        state.metadata_tags_by_file.get(&selected_file),
        Some(&vec![String::from("hat")])
    );
    assert_eq!(
        state.metadata_tags_by_file.get("other.wav"),
        Some(&vec![String::from("seq")])
    );
    assert_eq!(state.context_menu, None);
    assert_eq!(
        state.sample_status,
        "Deleted tag oneshot from 2 assignment(s)"
    );
}

#[test]
fn metadata_tag_context_delete_rejects_locked_playback_tags() {
    let (mut state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.context_menu = Some(super::super::super::BrowserContextMenu {
        kind: super::super::super::BrowserContextTargetKind::MetadataTag,
        path: PathBuf::new(),
        source_id: None,
        metadata_tag: Some(String::from("loop")),
        collection: None,
        anchor: Point::new(12.0, 24.0),
        title: String::from("loop"),
    });

    state.delete_context_metadata_tag(&mut ui::UpdateContext::default());

    assert_eq!(state.sample_status, "Playback Type tags are locked");
}

#[test]
fn default_gui_tag_library_uses_custom_dictionary_categories() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(selected_file, vec![String::from("deep-kick")]);
    state
        .metadata_tag_dictionary
        .insert(String::from("deep-kick"), String::from("sound-type"));
    state.metadata_tag_library_open = true;

    let frame = super::super::super::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("Sound Type (1)"));
    assert!(frame.paint_plan.contains_text("deep-kick"));
    assert!(frame.paint_plan.contains_text("Character"));
    assert!(!frame.paint_plan.contains_text("Character (1)"));
}
