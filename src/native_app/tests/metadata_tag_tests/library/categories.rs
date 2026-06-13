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
        &mut ui::UpdateContext::default(),
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
