use super::super::*;

#[test]
fn metadata_tag_field_background_click_focuses_tag_input() {
    let (state, _source_root, _selected_file) = native_app_state_with_temp_sample("tag-target.wav");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let input_rect = metadata_tag_text_input(&frame)
        .map(|input| input.rect)
        .expect("tag input should paint");
    let input_id = text_input_widget_id(&frame).expect("tag input widget id");
    assert!(
        input_rect.width() > 160.0,
        "tag input should expose a broad click target, got {input_rect:?}"
    );
    let point = Point::new(input_rect.min.x + 10.0, input_rect.center().y);

    runtime.dispatch_primary_click(point);
    let target = runtime.widget_at(point);
    let target_rect = target.and_then(|widget_id| {
        runtime
            .frame_with_default_theme()
            .paint_plan
            .first_widget_rect(widget_id)
    });

    assert_eq!(
        runtime.focused_widget(),
        Some(input_id),
        "clicking the visible tag-entry field should focus the embedded tag input; target={target:?} target_rect={target_rect:?} input_rect={input_rect:?}"
    );
}

#[test]
fn metadata_section_sits_flush_against_bottom_status_bar() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("kick")]);

    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    let metadata_rect = frame
        .paint_plan
        .first_widget_rect(
            crate::native_app::test_support::metadata_sidebar::METADATA_SIDEBAR_PANEL_ID,
        )
        .expect("metadata panel should paint");
    let status_text_rect = frame
        .paint_plan
        .first_text_rect("1 sample")
        .expect("bottom status bar should paint selected sample count");
    let gap_to_status_text = status_text_rect.min.y - metadata_rect.max.y;

    assert!(
        (0.0..=6.0).contains(&gap_to_status_text),
        "metadata panel should sit directly against the bottom status bar, metadata={metadata_rect:?}, status_text={status_text_rect:?}, gap={gap_to_status_text}"
    );
}

#[test]
fn metadata_section_keeps_configured_height_without_selected_file() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        false,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &[],
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));
    let metadata_rect = frame
        .paint_plan
        .first_widget_rect(
            crate::native_app::test_support::metadata_sidebar::METADATA_SIDEBAR_PANEL_ID,
        )
        .expect("metadata panel should stay in the sidebar without a selected file");

    assert_eq!(metadata_rect.height(), browser.metadata_panel_height());
}

#[test]
fn metadata_tag_library_toggle_sits_inline_after_add_tag_input() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let view = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &[],
        &[],
        None,
    );
    let layout = view.view_layout_at_size(Vector2::new(260.0, 620.0));
    let input_rect = layout
        .rects
        .get(&crate::native_app::test_support::metadata_sidebar::METADATA_TAG_INPUT_ID)
        .expect("metadata tag input should be laid out");
    let toggle_rect = layout
        .rects
        .get(&crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID)
        .expect("metadata tag library toggle should be laid out");

    assert!(
        toggle_rect.min.x > input_rect.max.x,
        "library toggle should sit to the right of add-tag input, input={input_rect:?}, toggle={toggle_rect:?}"
    );
    assert!(
        (toggle_rect.center().y - input_rect.center().y).abs() < 0.01,
        "library toggle should align vertically with the add-tag input, input={input_rect:?}, toggle={toggle_rect:?}"
    );
    assert!(
        toggle_rect.width() > 12.0,
        "library toggle should be larger than the old thin header affordance, toggle={toggle_rect:?}"
    );
}

#[test]
fn metadata_resize_header_uses_full_header_hit_target() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let view = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &[],
        &[],
        None,
    );
    let layout = view.view_layout_at_size(Vector2::new(260.0, 620.0));
    let panel_rect = layout
        .rects
        .get(&crate::native_app::test_support::metadata_sidebar::METADATA_SIDEBAR_PANEL_ID)
        .expect("metadata panel should be laid out");
    let header_rect = layout
        .rects
        .get(&crate::native_app::test_support::metadata_sidebar::METADATA_RESIZE_HEADER_ID)
        .expect("metadata resize header should be laid out");
    let drag =
        DragHandleMessage::started(Point::new(header_rect.center().x, header_rect.center().y));

    assert!(
        header_rect.width() >= panel_rect.width() - 12.0,
        "metadata resize hit target should cover the header width, panel={panel_rect:?}, header={header_rect:?}"
    );
    assert_eq!(
        crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
            &browser,
            260.0,
            true,
            "",
            &[],
            None,
            "add tag",
            None,
            &[],
            &[],
            &[],
            None,
        )
        .view_dispatch_widget_output(
            crate::native_app::test_support::metadata_sidebar::METADATA_RESIZE_HEADER_ID,
            ui::WidgetOutput::typed(drag),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::FolderBrowser(
                crate::native_app::test_support::state::FolderBrowserMessage::ResizeMetadataPanel(
                    drag
                )
            )
        )
    );
}

#[test]
fn metadata_section_collapses_to_header_only_height() {
    let (mut state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ResizeMetadataPanel(
            radiant::widgets::DragHandleMessage::double_activate(Point::new(0.0, 200.0)),
        ),
    );

    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    let metadata_rect = frame
        .paint_plan
        .first_widget_rect(
            crate::native_app::test_support::metadata_sidebar::METADATA_SIDEBAR_PANEL_ID,
        )
        .expect("metadata panel should paint");

    assert_eq!(
        metadata_rect.height(),
        crate::native_app::sample_library::folder_browser::test_support::COLLAPSED_METADATA_PANEL_HEIGHT
    );
}

#[test]
fn folder_browser_metadata_tag_field_renders_pending_category_prompt() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let completion_options = vec![super::super::super::metadata::MetadataTagCompletionOption {
        tag: String::from("Sound Type"),
        category: "Group",
        selected: true,
    }];
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "sound",
        &[],
        Some("deep-kick"),
        "select group/parent tag",
        Some("-type"),
        completion_options.as_slice(),
        &[],
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    assert!(frame.paint_plan.contains_text("deep-kick ->"));
    let pending_tag_rect = frame
        .paint_plan
        .first_text_rect("deep-kick ->")
        .expect("pending tag should paint");
    let category_input = metadata_tag_text_input(&frame).expect("category input should paint");
    assert_eq!(category_input.state.value, "sound");
    assert_eq!(category_input.state.selection_anchor, 5);
    assert_eq!(category_input.state.caret, 5);
    assert_eq!(category_input.completion_suffix.as_deref(), Some("-type"));
    assert!(
        category_input.rect.min.x > pending_tag_rect.max.x,
        "category input should stay on the same row after the pending tag arrow"
    );
}

#[test]
fn folder_browser_metadata_tag_input_moves_to_next_row_when_crowded() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "wow",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    let first_tag_y = frame
        .paint_plan
        .fill_rects()
        .find_map(|fill| ((fill.rect.height() - 18.0).abs() < 0.01).then_some(fill.rect.min.y));
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = metadata_tag_text_input(&frame).map(|input| input.rect);
    let input_rect = input_rect.expect("tag input should paint");
    let toggle_rect = frame
        .paint_plan
        .first_widget_rect(
            crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID,
        )
        .expect("tag library toggle should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(
        (toggle_rect.center().y - input_rect.center().y).abs() < 0.01,
        "tag library toggle should stay aligned with wrapped input, input={input_rect:?}, toggle={toggle_rect:?}"
    );
    assert!(input_rect.max.x <= 260.0);
}

#[test]
fn folder_browser_metadata_tag_input_keeps_identity_when_wrapping_rows() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let short_tags = vec![String::from("kick")];
    let crowded_tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let short_frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "wow",
        &[],
        None,
        "add tag",
        None,
        &[],
        &short_tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));
    let crowded_frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "wow",
        &[],
        None,
        "add tag",
        None,
        &[],
        &crowded_tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    let short_input = text_input_widget_id(&short_frame).expect("short tag field input");
    let crowded_input = text_input_widget_id(&crowded_frame).expect("crowded tag field input");

    assert_eq!(short_input, crowded_input);
}

#[test]
fn folder_browser_metadata_tag_input_wraps_after_full_tag_row() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![
        String::from("yay"),
        String::from("cool-tag"),
        String::from("thing"),
        String::from("potato"),
    ];
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        450.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        &[],
        None,
    )
    .view_frame_at_size_with_default_theme(Vector2::new(450.0, 620.0));

    let first_tag_y = frame
        .paint_plan
        .fill_rects()
        .find_map(|fill| ((fill.rect.height() - 18.0).abs() < 0.01).then_some(fill.rect.min.y));
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = metadata_tag_text_input(&frame).map(|input| input.rect);
    let input_rect = input_rect.expect("tag input should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(input_rect.max.x <= 450.0);
}
