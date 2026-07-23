use super::super::*;

#[test]
fn folder_browser_sidebar_paints_filter_and_metadata_sections() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
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
    .view_frame_at_size_with_default_theme(Vector2::new(260.0, 620.0));

    assert!(!frame.paint_plan.contains_text("Filter"));
    assert!(frame.paint_plan.contains_text("TAGS"));
    assert!(!frame.paint_plan.contains_text("Metadata"));
    assert!(!frame.paint_plan.contains_text("Tags (1)"));
    assert!(!frame.paint_plan.contains_text("Tagging"));
    assert!(frame.paint_plan.contains_text("KICK ×"));
    assert!(!frame.paint_plan.contains_text("(1)"));
    let toggle_rect = frame
        .paint_plan
        .first_widget_rect(
            crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID,
        )
        .expect("metadata tag library toggle should paint");
    assert!(
        frame
            .paint_plan
            .svgs()
            .any(|svg| svg.rect.intersects(toggle_rect)),
        "metadata tag library disclosure icon should paint inside the compact toggle"
    );
}

#[test]
fn metadata_tag_library_toggle_exposes_state_aware_help_tooltip() {
    let (mut closed_state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    closed_state.ui.chrome.help_tooltips_enabled = true;
    closed_state.metadata.tag_library_open = false;
    let closed_surface = crate::native_app::test_support::state::view(&closed_state).into_surface();
    let closed_tooltip = closed_surface
        .find_widget(
            crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID,
        )
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(closed_tooltip, Some("Show tag library"));

    let (mut open_state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    open_state.ui.chrome.help_tooltips_enabled = true;
    open_state.metadata.tag_library_open = true;
    let open_surface = crate::native_app::test_support::state::view(&open_state).into_surface();
    let open_tooltip = open_surface
        .find_widget(
            crate::native_app::test_support::metadata_sidebar::METADATA_TAG_LIBRARY_TOGGLE_ID,
        )
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(open_tooltip, Some("Hide tag library"));
}

#[test]
fn folder_browser_metadata_selected_tag_chip_uses_strong_accent_style() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![String::from("hat")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        260.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        &[],
        Some("hat"),
    )
    .view_frame_at_size(Vector2::new(260.0, 620.0), &theme);

    let tag_text = frame
        .paint_plan
        .first_text_run("HAT ×")
        .expect("selected tag chip should paint");
    assert_eq!(tag_text.color, theme.text_primary);
}

#[test]
fn clicking_metadata_tag_chip_selects_it_in_sidebar() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let tag_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("HAT ×")
        .expect("metadata tag chip should paint");
    let point = tag_rect.center();

    runtime.dispatch_event(Event::primary_press(point));
    runtime.dispatch_message(crate::native_app::test_support::state::GuiMessage::Frame);
    runtime.dispatch_event(Event::primary_release(point));

    assert_eq!(
        runtime.bridge().state().metadata.selected_tag.as_deref(),
        Some("hat")
    );
}

#[test]
fn right_clicking_metadata_tag_chip_opens_delete_context_menu() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state
        .metadata
        .tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let tag_rect = runtime
        .frame_with_default_theme()
        .paint_plan
        .first_text_rect("HAT ×")
        .expect("metadata tag chip should paint");
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
    assert_eq!(menu.metadata_tag.as_deref(), Some("hat"));
}

#[test]
fn metadata_tag_chips_display_playback_tags_first() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("tag-target.wav");
    state.metadata.tags_by_file.insert(
        selected_file,
        vec![
            String::from("hat"),
            String::from("warm"),
            String::from("loop"),
            String::from("one-shot"),
        ],
    );

    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    let loop_rect = frame
        .paint_plan
        .first_text_rect("LOOP ×")
        .expect("loop tag should paint");
    let one_shot_rect = frame
        .paint_plan
        .first_text_rect("ONE-SHOT ×")
        .expect("one-shot tag should paint");
    let hat_rect = frame
        .paint_plan
        .first_text_rect("HAT ×")
        .expect("hat tag should paint");
    let warm_rect = frame
        .paint_plan
        .first_text_rect("WARM ×")
        .expect("warm tag should paint");

    let is_before = |left: Rect, right: Rect| {
        left.min.y < right.min.y
            || ((left.min.y - right.min.y).abs() < 0.01 && left.min.x < right.min.x)
    };
    assert!(is_before(loop_rect, hat_rect));
    assert!(is_before(one_shot_rect, hat_rect));
    assert!(is_before(hat_rect, warm_rect));
}

#[test]
fn metadata_tag_chips_show_mixed_tags_for_multi_selection() {
    let source_root = tempfile::tempdir().expect("source root");
    let first = source_root.path().join("first.wav");
    let second = source_root.path().join("second.wav");
    fs::write(&first, []).expect("first sample");
    fs::write(&second, []).expect("second sample");
    let source = wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf());
    let first_id = first.display().to_string();
    let second_id = second.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[source]);
    state.library.folder_browser.select_file(first_id.clone());
    state.library.folder_browser.select_file_with_modifiers(
        second_id.clone(),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    state
        .metadata
        .tags_by_file
        .insert(first_id, vec![String::from("bass")]);
    state
        .metadata
        .tags_by_file
        .insert(second_id, vec![String::from("dry")]);
    let theme = radiant::theme::ThemeTokens::default();
    let expected = radiant::widgets::resolve_widget_visual_tokens(
        &theme,
        crate::native_app::metadata::metadata_tag_pill_selection_style(
            "sound-type",
            crate::native_app::metadata::MetadataTagSelectionState::Mixed,
        ),
        radiant::widgets::WidgetState::default(),
    );

    let frame = crate::native_app::test_support::state::view(&state)
        .view_frame_at_size(Vector2::new(900.0, 620.0), &theme);
    let bass_rect = frame
        .paint_plan
        .first_text_rect("BASS ×")
        .expect("mixed bass tag should paint");

    assert!(frame.paint_plan.contains_text("DRY ×"));
    assert!(
        frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == expected.fill && fill.rect.intersects(bass_rect)),
        "mixed inline tag should paint the partial-assignment pill fill"
    );
}

#[test]
fn metadata_tag_chips_group_by_target_category_order_and_color() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![
        String::from("warm"),
        String::from("artist1"),
        String::from("dorian"),
        String::from("hat"),
        String::from("loop"),
    ];
    let categories = vec![
        super::super::super::metadata::MetadataTagDisplayCategory {
            tag: String::from("warm"),
            category_id: "character",
        },
        super::super::super::metadata::MetadataTagDisplayCategory {
            tag: String::from("artist1"),
            category_id: "prefix",
        },
        super::super::super::metadata::MetadataTagDisplayCategory {
            tag: String::from("dorian"),
            category_id: "tuning-scale",
        },
        super::super::super::metadata::MetadataTagDisplayCategory {
            tag: String::from("hat"),
            category_id: "sound-type",
        },
        super::super::super::metadata::MetadataTagDisplayCategory {
            tag: String::from("loop"),
            category_id: "playback-type",
        },
    ];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = crate::native_app::test_support::metadata_sidebar::library_sidebar_view(
        &browser,
        600.0,
        true,
        "",
        &[],
        None,
        "add tag",
        None,
        &[],
        &tags,
        categories.as_slice(),
        None,
    )
    .view_frame_at_size(Vector2::new(600.0, 620.0), &theme);

    let loop_rect = frame
        .paint_plan
        .first_text_rect("LOOP ×")
        .expect("loop tag should paint");
    let hat_rect = frame
        .paint_plan
        .first_text_rect("HAT ×")
        .expect("hat tag should paint");
    let warm_rect = frame
        .paint_plan
        .first_text_rect("WARM ×")
        .expect("warm tag should paint");
    let artist_rect = frame
        .paint_plan
        .first_text_rect("ARTIST1 ×")
        .expect("prefix tag should paint");
    let dorian_rect = frame
        .paint_plan
        .first_text_rect("DORIAN ×")
        .expect("tuning tag should paint");

    assert!(loop_rect.min.x < hat_rect.min.x);
    assert!(hat_rect.min.x < warm_rect.min.x);
    assert!(warm_rect.min.x < artist_rect.min.x);
    assert!(artist_rect.min.x < dorian_rect.min.x);

    assert_eq!(
        frame.paint_plan.first_text_color("LOOP ×"),
        Some(theme.text_primary)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("HAT ×"),
        Some(theme.text_primary)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("WARM ×"),
        Some(theme.text_primary)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("ARTIST1 ×"),
        Some(theme.text_primary)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("DORIAN ×"),
        Some(theme.text_primary)
    );
}
