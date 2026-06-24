use super::*;

#[test]
fn default_gui_tag_library_category_headers_collapse_groups() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    state.metadata.tag_library_open = true;

    state.apply_message(
        toggle_metadata_tag_category(String::from("sound-type")),
        &mut ui::UiUpdateContext::default(),
    );

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.contains_text("Sound Type (1)"));
    assert!(
        !frame
            .paint_plan
            .contains_text_after_x("hat", DEFAULT_FOLDER_WIDTH)
    );
}

#[test]
fn default_gui_tag_library_category_chevron_click_toggles_group() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    state.metadata.tag_library_open = true;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let header_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_run_after_x("Sound Type (1)", DEFAULT_FOLDER_WIDTH)
        .expect("tag library sound-type header should paint")
        .rect;
    let chevron_point = Point::new(header_rect.min.x - 12.0, header_rect.center().y);

    runtime.dispatch_input_at(chevron_point, WidgetInput::primary_press(chevron_point));
    runtime.dispatch_input_at(chevron_point, WidgetInput::primary_release(chevron_point));

    let frame = runtime.frame_with_default_theme();
    assert!(frame.paint_plan.contains_text("Sound Type (1)"));
    assert!(
        !frame
            .paint_plan
            .contains_text_after_x("hat", DEFAULT_FOLDER_WIDTH)
    );
}
