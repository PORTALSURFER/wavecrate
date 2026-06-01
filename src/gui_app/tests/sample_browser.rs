use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::IntoView,
    runtime::PaintPrimitive,
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput},
};

#[test]
fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 22.0));
    let mut hit_target = crate::gui_app::sample_browser_view::SampleFileHitTarget::new(
        false, false, false, false, false,
    );

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::primary_press(Point::new(24.0, 10.0)),),
        None
    );

    let mut refreshed_hit_target = crate::gui_app::sample_browser_view::SampleFileHitTarget::new(
        false, false, false, false, false,
    );
    refreshed_hit_target.common_mut().state = hit_target.common().state;
    let output = refreshed_hit_target
        .handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(24.0, 10.0),
                PointerButton::Primary,
                PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            ),
        )
        .expect("sample row should activate after a frame refresh");

    assert_eq!(
        output.typed_copied::<crate::gui_app::sample_browser_view::SampleFileHitMessage>(),
        Some(
            crate::gui_app::sample_browser_view::SampleFileHitMessage::Activate(PointerModifiers {
                command: true,
                shift: true,
                ..Default::default()
            })
        )
    );
    assert!(!refreshed_hit_target.common().state.pressed);
}

#[test]
fn sample_browser_frame_paints_column_and_file_text() {
    let mut state = crate::gui_app::GuiAppState::load_default().expect("default state loads");
    let expected_stem = state
        .folder_browser
        .selected_audio_files()
        .first()
        .map(|file| file.stem.clone())
        .expect("default assets include an audio sample");
    let frame = crate::gui_app::sample_browser(&mut state, false)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let texts = frame
        .paint_plan
        .text_runs()
        .map(|text| text.text.as_str().to_string())
        .collect::<Vec<_>>();

    assert!(
        texts.iter().any(|text| text.starts_with("Name")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with(&expected_stem)),
        "{texts:?}"
    );
}

#[test]
fn sample_browser_rows_match_keyboard_scroll_stride() {
    let mut state = crate::gui_app::GuiAppState::load_default().expect("default state loads");
    let expected_names = state
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::gui_app::sample_browser(&mut state, false)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let mut row_tops = frame
        .paint_plan
        .text_runs()
        .filter(|text| {
            expected_names
                .iter()
                .any(|name| text.text.as_str().starts_with(name))
        })
        .map(|text| text.rect.min.y)
        .collect::<Vec<_>>();
    row_tops.sort_by(|a, b| a.total_cmp(b));
    row_tops.dedup_by(|a, b| (*a - *b).abs() < 0.5);

    assert!(row_tops.len() >= 2, "{row_tops:?}");
    assert!(
        row_tops.windows(2).all(|pair| {
            ((pair[1] - pair[0]) - crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT).abs() < 0.5
        }),
        "{row_tops:?}"
    );
}

#[test]
fn sample_browser_keyboard_scroll_keeps_two_context_rows() {
    assert_eq!(crate::gui_app::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, 2);
    assert_eq!(crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT, 22.0);
}

#[test]
fn selected_sample_browser_row_paints_strong_fill_and_left_marker() {
    let widget = crate::gui_app::sample_browser_view::SampleFileHitTarget::new(
        true, false, false, false, false,
    );
    let bounds = Rect::from_min_size(Point::new(12.0, 8.0), Vector2::new(240.0, 22.0));
    let primitives = widget.paint_primitives_with_defaults(bounds);
    let fills = primitives
        .iter()
        .filter_map(PaintPrimitive::fill_rect)
        .collect::<Vec<_>>();

    assert!(fills.iter().any(|fill| fill.rect == bounds
        && fill.color
            == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            }));
    assert!(fills.iter().any(|fill| {
        fill.color
            == Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            }
            && fill.rect.width() <= 3.5
    }));
}

#[test]
fn sample_browser_row_hover_paints_bright_background_without_marker() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 22.0));
    let mut hit_target = crate::gui_app::sample_browser_view::SampleFileHitTarget::new(
        false, false, false, false, false,
    );

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0)),),
        None
    );

    let primitives = hit_target.paint_primitives_with_defaults(bounds);
    let fills = primitives
        .iter()
        .filter_map(PaintPrimitive::fill_rect)
        .collect::<Vec<_>>();

    assert!(
        fills.iter().any(|fill| fill.rect == bounds
            && fill.color
                == Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 24,
                }),
        "{fills:?}"
    );
}

#[test]
fn full_gui_frame_places_sample_browser_text_inside_visible_area() {
    let mut state = crate::gui_app::GuiAppState::load_default().expect("default state loads");
    let expected_names = state
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::gui_app::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(1517.0, 758.0));
    let sample_texts = frame
        .paint_plan
        .text_runs()
        .filter(|text| {
            text.text.as_str() == "Name"
                || expected_names
                    .iter()
                    .any(|name| text.text.as_str().starts_with(name))
        })
        .map(|text| (text.text.as_str().to_string(), text.rect, text.baseline))
        .collect::<Vec<_>>();

    assert!(!sample_texts.is_empty(), "{sample_texts:?}");
    assert!(
        sample_texts.iter().any(|(_, rect, baseline)| {
            rect.width() > 20.0
                && rect.height() >= 10.0
                && rect.min.x >= 280.0
                && rect.min.y >= 320.0
                && rect.max.y <= 730.0
                && baseline.is_some()
        }),
        "{sample_texts:?}"
    );
}
