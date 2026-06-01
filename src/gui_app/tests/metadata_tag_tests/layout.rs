use super::super::*;

#[test]
fn metadata_tag_field_background_click_focuses_tag_input() {
    let (state, _source_root, _selected_file) = gui_state_with_temp_sample("tag-target.wav");
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| radiant::runtime::UiSurface::new(super::super::super::view(state).into_node()),
        |state, message| state.apply_message(message, &mut ui::UpdateContext::default()),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&radiant::theme::ThemeTokens::default());
    let input_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.rect),
            _ => None,
        })
        .expect("tag input should paint");
    let input_id = text_input_widget_id(&frame).expect("tag input widget id");
    assert!(
        input_rect.width() > 160.0,
        "tag input should expose a broad click target, got {input_rect:?}"
    );
    let point = Point::new(
        input_rect.min.x + 10.0,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );

    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    let target = runtime.widget_at(point);
    let target_rect = target.and_then(|widget_id| {
        runtime
            .frame(&radiant::theme::ThemeTokens::default())
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
fn folder_browser_metadata_tag_field_renders_pending_category_prompt() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let completion_options = vec![
        super::super::super::metadata_tags::MetadataTagCompletionOption {
            tag: String::from("Sound Type"),
            category: "Group",
            selected: true,
        },
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    assert!(frame_has_text(&frame, "deep-kick ->"));
    assert!(frame_has_text(&frame, "Sound Type"));
    assert!(frame_has_text(&frame, "Group"));
    let pending_tag_rect = text_rect(&frame, "deep-kick ->").expect("pending tag should paint");
    let category_input = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input),
            _ => None,
        })
        .expect("category input should paint");
    assert_eq!(category_input.state.value, "sound");
    assert_eq!(category_input.state.selection_anchor, 5);
    assert_eq!(category_input.state.caret, 5);
    assert_eq!(category_input.completion_suffix.as_deref(), Some("-type"));
    assert!(
        category_input.rect.min.x > pending_tag_rect.max.x,
        "category input should stay on the same row after the pending tag arrow"
    );
    let sound_type_rect = text_rect(&frame, "Sound Type").expect("completion option should paint");
    assert!(
        sound_type_rect.max.y < pending_tag_rect.min.y,
        "completion popup should expand upward above the tag input"
    );
}

#[test]
fn folder_browser_metadata_tag_input_moves_to_next_row_when_crowded() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let first_tag_y = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::FillRect(fill) = primitive
            && (fill.rect.height() - 18.0).abs() < 0.01
        {
            return Some(fill.rect.min.y);
        }
        None
    });
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::TextInput(input) = primitive {
            return Some(input.rect);
        }
        None
    });
    let input_rect = input_rect.expect("tag input should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(input_rect.max.x <= 260.0);
}

#[test]
fn folder_browser_metadata_tag_input_keeps_identity_when_wrapping_rows() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let short_tags = vec![String::from("kick")];
    let crowded_tags = vec![
        String::from("test"),
        String::from("another"),
        String::from("cool-tag"),
    ];
    let short_frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let crowded_frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(260.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let short_input = text_input_widget_id(&short_frame).expect("short tag field input");
    let crowded_input = text_input_widget_id(&crowded_frame).expect("crowded tag field input");

    assert_eq!(short_input, crowded_input);
}

#[test]
fn folder_browser_metadata_tag_input_wraps_after_full_tag_row() {
    let browser = super::super::super::FolderBrowserState::load_default();
    let tags = vec![
        String::from("yay"),
        String::from("cool-tag"),
        String::from("thing"),
        String::from("potato"),
    ];
    let frame = radiant::runtime::UiSurface::new(
        super::super::super::folder_browser::folder_browser_view(
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
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(450.0, 620.0)),
        &radiant::theme::ThemeTokens::default(),
    );

    let first_tag_y = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::FillRect(fill) = primitive
            && (fill.rect.height() - 18.0).abs() < 0.01
        {
            return Some(fill.rect.min.y);
        }
        None
    });
    let first_tag_y = first_tag_y.expect("tag pill should paint in the tag field");
    let input_rect = frame.paint_plan.primitives.iter().find_map(|primitive| {
        if let PaintPrimitive::TextInput(input) = primitive {
            return Some(input.rect);
        }
        None
    });
    let input_rect = input_rect.expect("tag input should paint");

    assert!(input_rect.min.y > first_tag_y);
    assert!(input_rect.max.x <= 450.0);
}
