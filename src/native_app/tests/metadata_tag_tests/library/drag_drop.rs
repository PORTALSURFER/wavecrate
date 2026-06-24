use super::*;

#[test]
fn default_gui_tag_library_drag_moves_tag_between_categories() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);

    state.drag_metadata_tag(
        String::from("bass"),
        DragHandleMessage::started(Point::new(10.0, 10.0)),
        &mut ui::UiUpdateContext::default(),
    );
    state.drop_metadata_tag_on_category(
        String::from("character"),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(
        state
            .metadata
            .tag_dictionary
            .get("bass")
            .map(String::as_str),
        Some("character")
    );
    assert_eq!(state.ui.status.sample, "Moved tag bass to Character");
}

#[test]
fn default_gui_tag_library_rejects_dragging_locked_playback_tags() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");

    state.drag_metadata_tag(
        String::from("one-shot"),
        DragHandleMessage::started(Point::new(10.0, 10.0)),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.metadata.tag_drag, None);
    assert_eq!(state.metadata.tag_dictionary.get("one-shot"), None);
    assert_eq!(state.ui.status.sample, "Playback Type tags are locked");
}

#[test]
fn default_gui_tag_library_clear_drop_message_preserves_current_target_only() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tag_drag = Some(String::from("bass"));
    state.metadata.tag_drop_hover = Some(String::from("character"));

    state.apply_message(
        clear_metadata_tag_drop_category_unless(String::from("character")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.metadata.tag_drop_hover.as_deref(), Some("character"));

    state.apply_message(
        clear_metadata_tag_drop_category_unless(String::from("playback-type")),
        &mut ui::UiUpdateContext::default(),
    );

    assert_eq!(state.metadata.tag_drop_hover, None);
}

#[test]
fn default_gui_tag_library_pointer_hover_over_noncandidate_badge_clears_target() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);
    state.metadata.tag_library_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let bass_point = frame
        .paint_plan
        .first_text_rect("bass")
        .expect("bass tag should paint")
        .center();
    let character_point = frame
        .paint_plan
        .first_text_rect("Character")
        .expect("character header should paint")
        .center();
    let locked_playback_tag_point = frame
        .paint_plan
        .first_text_rect("loop")
        .expect("locked playback tag should paint")
        .center();

    runtime.dispatch_event(Event::primary_press(bass_point));
    runtime.dispatch_event(Event::pointer_move(Point::new(
        bass_point.x + 8.0,
        bass_point.y + 2.0,
    )));
    runtime.dispatch_event(Event::pointer_move(character_point));

    assert_eq!(
        runtime.bridge().state().metadata.tag_drop_hover.as_deref(),
        Some("character")
    );

    runtime.dispatch_event(Event::pointer_move(locked_playback_tag_point));

    assert_eq!(runtime.bridge().state().metadata.tag_drop_hover, None);
}

#[test]
fn default_gui_tag_library_pointer_drag_drops_tag_on_category_header() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(String::from("other.wav"), vec![String::from("bass")]);
    state.metadata.tag_library_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
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
            .metadata
            .tag_dictionary
            .get("bass")
            .map(String::as_str),
        Some("character")
    );
}
