use super::*;

#[test]
fn folder_browser_metadata_tag_field_renders_completion_suffix_without_overlay_options() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let completion_options = vec![
        crate::native_app::metadata::MetadataTagCompletionOption {
            tag: String::from("kick"),
            category: "Sound Type",
            selected: true,
        },
        crate::native_app::metadata::MetadataTagCompletionOption {
            tag: String::from("kicker"),
            category: "Character",
            selected: false,
        },
    ];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "ki",
        &[String::from("kick")],
        None,
        "add tag",
        Some("ck"),
        completion_options.as_slice(),
        &[String::from("warm")],
        &[],
        None,
    )
    .view_frame_at_size(Vector2::new(260.0, 620.0), &theme);

    assert!(frame.paint_plan.contains_text("kick"));
    let tag_input = metadata_tag_text_input(&frame).expect("tag input should paint");
    assert_eq!(tag_input.state.value, "ki");
    assert_eq!(tag_input.state.selection_anchor, 2);
    assert_eq!(tag_input.state.caret, 2);
    assert_eq!(tag_input.completion_suffix.as_deref(), Some("ck"));
    assert_eq!(tag_input.completion_color, theme.text_muted);
    assert!(!frame.paint_plan.contains_text("Sound Type"));
    assert!(!frame.paint_plan.contains_text("kicker"));
    assert!(!frame.paint_plan.contains_text("Character"));
    assert!(!frame.paint_plan.contains_text("Tab kick"));
    assert!(frame.paint_plan.contains_text("warm"));
    assert!(
        frame
            .paint_plan
            .text_inputs()
            .any(|input| input.rect.height() <= 14.0)
    );
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| (fill.rect.height() - 18.0).abs() < 0.01)
    );
}

#[test]
fn folder_browser_metadata_category_completion_renders_above_tag_input() {
    let (baseline_state, _baseline_source_root, _baseline_selected_file) =
        native_app_state_with_temp_sample("baseline-tag-target.wav");
    let baseline_frame = crate::native_app::test_support::state::view(&baseline_state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    let baseline_tag_input =
        metadata_tag_text_input(&baseline_frame).expect("baseline tag input should paint");

    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("category-tag-target.wav");
    state.metadata.tag_input_mode =
        crate::native_app::test_support::waveform::MetadataTagInputMode::Category {
            pending_tag: String::from("new-tag"),
        };
    state.metadata.tag_draft.clear();

    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    let tag_input = metadata_tag_text_input(&frame).expect("tag input should paint");
    let final_option = frame
        .paint_plan
        .first_text_rect("Tuning/Scale")
        .expect("final category option should paint");

    assert!(
        final_option.max.y <= tag_input.rect.min.y,
        "category completion popup should fit above the tag input, option {final_option:?}, input {:?}",
        tag_input.rect
    );
    assert_eq!(
        tag_input.rect.min.y, baseline_tag_input.rect.min.y,
        "floating category completion should not expand or shift the tags section"
    );
}
