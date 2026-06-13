use super::*;

#[test]
fn default_gui_tag_library_opens_beside_library_sidebar() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        selected_file.clone(),
        vec![String::from("hat"), String::from("seq")],
    );
    state.metadata.tags_by_file.insert(
        String::from("other.wav"),
        vec![String::from("bass"), String::from("hat")],
    );
    state.metadata.tag_library_open = true;

    let frame = crate::native_app::test_support::state::view(&mut state)
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
fn default_gui_tag_library_paints_inactive_playback_tags_as_neutral_pills() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tag_library_open = true;
    let theme = radiant::theme::ThemeTokens::default();
    let expected = radiant::widgets::resolve_widget_visual_tokens(
        &theme,
        crate::native_app::metadata::metadata_tag_pill_style("playback-type", false),
        radiant::widgets::WidgetState::default(),
    );

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size(Vector2::new(900.0, 620.0), &theme);

    assert_eq!(
        frame.paint_plan.first_text_color("one-shot"),
        Some(expected.foreground)
    );
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == expected.fill),
        "inactive playback tag should paint the neutral warning pill fill"
    );
}

#[test]
fn default_gui_tag_library_paints_applied_playback_tags_as_active_pills() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("one-shot")]);
    state.metadata.tag_library_open = true;
    let theme = radiant::theme::ThemeTokens::default();
    let expected = radiant::widgets::resolve_widget_visual_tokens(
        &theme,
        crate::native_app::metadata::metadata_tag_pill_style("playback-type", true),
        radiant::widgets::WidgetState {
            active: true,
            ..radiant::widgets::WidgetState::default()
        },
    );

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size(Vector2::new(900.0, 620.0), &theme);

    assert_eq!(
        frame
            .paint_plan
            .first_text_run_after_x("one-shot", DEFAULT_FOLDER_WIDTH)
            .map(|run| run.color),
        Some(expected.foreground)
    );
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == expected.fill),
        "applied playback tag should paint the active warning pill fill"
    );
}

#[test]
fn default_gui_tag_library_uses_custom_dictionary_categories() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("deep-kick")]);
    state
        .metadata
        .tag_dictionary
        .insert(String::from("deep-kick"), String::from("sound-type"));
    state.metadata.tag_library_open = true;

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("Sound Type (1)"));
    assert!(frame.paint_plan.contains_text("deep-kick"));
    assert!(frame.paint_plan.contains_text("Character"));
    assert!(!frame.paint_plan.contains_text("Character (1)"));
}
