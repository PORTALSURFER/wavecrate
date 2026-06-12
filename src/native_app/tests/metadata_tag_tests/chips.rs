use super::super::*;

#[test]
fn folder_browser_sidebar_paints_filter_and_metadata_sections() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
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

    assert!(frame.paint_plan.contains_text("Filter"));
    assert!(frame.paint_plan.contains_text("Tags"));
    assert!(!frame.paint_plan.contains_text("Metadata"));
    assert!(!frame.paint_plan.contains_text("Tags (1)"));
    assert!(!frame.paint_plan.contains_text("Tagging"));
    assert!(frame.paint_plan.contains_text("kick"));
    let tag_count = frame
        .paint_plan
        .first_text_rect("(1)")
        .expect("metadata tag count should paint");
    assert!(
        frame
            .paint_plan
            .svgs()
            .any(|svg| svg.rect.min.x > tag_count.max.x && svg.rect.min.y <= tag_count.max.y),
        "metadata tag library disclosure icon should paint beside the tag count"
    );
}

#[test]
fn folder_browser_metadata_selected_tag_chip_uses_strong_accent_style() {
    let browser = crate::native_app::test_support::state::FolderBrowserState::load_default();
    let tags = vec![String::from("hat")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
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
        .first_text_run("hat")
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
        .first_text_rect("hat")
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
        .first_text_rect("hat")
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

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    let loop_rect = frame
        .paint_plan
        .first_text_rect("loop")
        .expect("loop tag should paint");
    let one_shot_rect = frame
        .paint_plan
        .first_text_rect("one-shot")
        .expect("one-shot tag should paint");
    let hat_rect = frame
        .paint_plan
        .first_text_rect("hat")
        .expect("hat tag should paint");
    let warm_rect = frame
        .paint_plan
        .first_text_rect("warm")
        .expect("warm tag should paint");

    assert!(loop_rect.min.x < hat_rect.min.x);
    assert!(one_shot_rect.min.x < hat_rect.min.x);
    assert!(hat_rect.min.x < warm_rect.min.x);
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
    let frame =
        crate::native_app::app_chrome::library_browser::library_sidebar::library_sidebar_view(
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
        .first_text_rect("loop")
        .expect("loop tag should paint");
    let hat_rect = frame
        .paint_plan
        .first_text_rect("hat")
        .expect("hat tag should paint");
    let warm_rect = frame
        .paint_plan
        .first_text_rect("warm")
        .expect("warm tag should paint");
    let artist_rect = frame
        .paint_plan
        .first_text_rect("artist1")
        .expect("prefix tag should paint");
    let dorian_rect = frame
        .paint_plan
        .first_text_rect("dorian")
        .expect("tuning tag should paint");

    assert!(loop_rect.min.x < hat_rect.min.x);
    assert!(hat_rect.min.x < warm_rect.min.x);
    assert!(warm_rect.min.x < artist_rect.min.x);
    assert!(artist_rect.min.x < dorian_rect.min.x);

    assert_eq!(
        frame.paint_plan.first_text_color("loop"),
        Some(theme.bg_primary)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("hat"),
        Some(theme.accent_mint)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("warm"),
        Some(theme.highlight_cyan)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("artist1"),
        Some(theme.accent_danger)
    );
    assert_eq!(
        frame.paint_plan.first_text_color("dorian"),
        Some(theme.text_muted)
    );
}
