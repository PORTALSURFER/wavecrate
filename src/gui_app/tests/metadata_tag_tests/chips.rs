use super::super::*;

#[test]
fn folder_browser_sidebar_paints_filter_and_metadata_sections() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![String::from("kick")];
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "Filter"));
    assert!(!frame_has_text(&frame, "Metadata"));
    assert!(!frame_has_text(&frame, "Tagging"));
    assert!(frame_has_text(&frame, "kick"));
    let tags_header = text_rect(&frame, "Tags (1)").expect("metadata tags header should paint");
    assert!(
        frame
            .paint_plan
            .svgs()
            .any(|svg| svg.rect.min.x > tags_header.max.x && svg.rect.min.y <= tags_header.max.y),
        "metadata tag library disclosure icon should paint beside the tags header"
    );
}

#[test]
fn folder_browser_metadata_selected_tag_chip_uses_strong_accent_style() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![String::from("hat")];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &theme,
    );

    let tag_text = frame
        .paint_plan
        .first_text_run("hat")
        .expect("selected tag chip should paint");
    assert_eq!(tag_text.color, theme.text_primary);
}

#[test]
fn clicking_metadata_tag_chip_selects_it_in_sidebar() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state
        .metadata_tags_by_file
        .insert(selected_file, vec![String::from("hat")]);
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let tag_rect = text_rect(
        &runtime.frame(&radiant::theme::ThemeTokens::default()),
        "hat",
    )
    .expect("metadata tag chip should paint");
    let point = tag_rect.center();

    runtime.dispatch_event(Event::primary_press(point));
    runtime.dispatch_message(super::super::super::GuiMessage::Frame);
    runtime.dispatch_event(Event::primary_release(point));

    assert_eq!(
        runtime.bridge().state().selected_metadata_tag.as_deref(),
        Some("hat")
    );
}

#[test]
fn metadata_tag_chips_display_playback_tags_first() {
    let (mut state, _source_root, selected_file) = gui_state_with_temp_sample("tag-target.wav");
    state.metadata_tags_by_file.insert(
        selected_file,
        vec![
            String::from("hat"),
            String::from("warm"),
            String::from("loop"),
            String::from("one-shot"),
        ],
    );

    let frame = radiant::runtime::UiSurface::new(super::super::super::view(&mut state).into_node())
        .frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(900.0, 620.0)),
            &radiant::theme::ThemeTokens::default(),
        );

    let loop_rect = text_rect(&frame, "loop").expect("loop tag should paint");
    let one_shot_rect = text_rect(&frame, "one-shot").expect("one-shot tag should paint");
    let hat_rect = text_rect(&frame, "hat").expect("hat tag should paint");
    let warm_rect = text_rect(&frame, "warm").expect("warm tag should paint");

    assert!(loop_rect.min.x < hat_rect.min.x);
    assert!(one_shot_rect.min.x < hat_rect.min.x);
    assert!(hat_rect.min.x < warm_rect.min.x);
}

#[test]
fn metadata_tag_chips_group_by_target_category_order_and_color() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![
        String::from("warm"),
        String::from("artist1"),
        String::from("dorian"),
        String::from("hat"),
        String::from("loop"),
    ];
    let categories = vec![
        super::super::super::metadata_tags::MetadataTagDisplayCategory {
            tag: String::from("warm"),
            category_id: "character",
        },
        super::super::super::metadata_tags::MetadataTagDisplayCategory {
            tag: String::from("artist1"),
            category_id: "prefix",
        },
        super::super::super::metadata_tags::MetadataTagDisplayCategory {
            tag: String::from("dorian"),
            category_id: "tuning-scale",
        },
        super::super::super::metadata_tags::MetadataTagDisplayCategory {
            tag: String::from("hat"),
            category_id: "sound-type",
        },
        super::super::super::metadata_tags::MetadataTagDisplayCategory {
            tag: String::from("loop"),
            category_id: "playback-type",
        },
    ];
    let theme = radiant::theme::ThemeTokens::default();
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(600.0, 620.0)),
        &theme,
    );

    let loop_rect = text_rect(&frame, "loop").expect("loop tag should paint");
    let hat_rect = text_rect(&frame, "hat").expect("hat tag should paint");
    let warm_rect = text_rect(&frame, "warm").expect("warm tag should paint");
    let artist_rect = text_rect(&frame, "artist1").expect("prefix tag should paint");
    let dorian_rect = text_rect(&frame, "dorian").expect("tuning tag should paint");

    assert!(loop_rect.min.x < hat_rect.min.x);
    assert!(hat_rect.min.x < warm_rect.min.x);
    assert!(warm_rect.min.x < artist_rect.min.x);
    assert!(artist_rect.min.x < dorian_rect.min.x);

    assert_eq!(text_color(&frame, "loop"), Some(theme.bg_primary));
    assert_eq!(text_color(&frame, "hat"), Some(theme.accent_mint));
    assert_eq!(text_color(&frame, "warm"), Some(theme.highlight_cyan));
    assert_eq!(text_color(&frame, "artist1"), Some(theme.accent_danger));
    assert_eq!(text_color(&frame, "dorian"), Some(theme.text_muted));
}
