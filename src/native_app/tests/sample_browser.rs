use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    prelude::IntoView,
    runtime::{Event, SurfaceFrame},
    widgets::{PointerButton, PointerModifiers, Widget, WidgetInput},
};
use std::fs;

use super::{native_app_state_with_temp_sample, native_runtime_for_tests};

const FOLDER_DROP_TARGET_FILL: Rgba8 = Rgba8::new(255, 130, 78, 220);

fn sample_hit_target(
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> crate::native_app::test_support::SampleFileHitTarget {
    crate::native_app::test_support::SampleFileHitTarget::new(
        String::from("sample.wav"),
        selected,
        drag_active,
        drag_source,
        cached,
    )
}

#[test]
fn sample_row_hit_target_survives_frame_refresh_between_press_and_release() {
    let bounds = Rect::from_size(160.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::primary_press(Point::new(24.0, 10.0)),),
        None
    );

    let mut refreshed_hit_target = sample_hit_target(false, false, false, false);
    refreshed_hit_target.synchronize_from_previous(&hit_target);
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
        output.typed_cloned::<crate::native_app::test_support::GuiMessage>(),
        Some(
            crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
                path: String::from("sample.wav"),
                modifiers: PointerModifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
            }
        )
    );
    assert!(!refreshed_hit_target.common().is_pressed());
}

#[test]
fn sample_browser_frame_paints_column_and_file_text() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let expected_stem = state
        .library
        .folder_browser
        .selected_audio_files()
        .first()
        .map(|file| file.stem.clone())
        .expect("default assets include an audio sample");
    let frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let texts = frame.paint_plan.text_label_strings();

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
fn sample_browser_column_drag_paints_drop_marker() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(292.0, 8.0)),
        ),
    );
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(420.0, 8.0)),
        ),
    );

    let frame = crate::native_app::test_support::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));

    assert!(frame.paint_plan.fill_rects().any(|fill| {
        fill.color == Rgba8::new(255, 160, 82, 230)
            && fill.rect.width() <= 2.5
            && fill.rect.height() >= 20.0
    }));
}

#[test]
fn sample_browser_header_paints_hover_affordance() {
    let state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let name_rect = frame
        .paint_plan
        .text_runs()
        .filter(|text| text.text.as_str().starts_with("Name"))
        .map(|text| text.rect)
        .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .expect("name column header should paint");
    let point = Point::new(name_rect.min.x + 8.0, name_rect.center().y);
    let target = runtime
        .widget_at(point)
        .expect("sample header should expose a sort/drag hit target");
    assert!(
        !frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(target),
        "sample header hit target should not paint button chrome before hover"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(target),
        "hovering the sample header should still route to the sort/drag hit target"
    );

    let hovered_frame = runtime.frame_with_default_theme();
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(target),
        "hovering a sample header should paint a subtle cell-sized affordance"
    );
}

#[test]
fn full_gui_column_drag_paints_pointer_preview() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(600.0, 320.0)),
        ),
    );
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(620.0, 320.0)),
        ),
    );

    let frame = crate::native_app::test_support::view(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));

    assert!(frame.paint_plan.text_runs().any(|text| {
        text.text == "Rating" && text.rect.min.x >= 620.0 && text.rect.min.y >= 330.0
    }));
}

#[test]
fn sample_column_resize_updates_rendered_row_layout_without_sorting() {
    let (state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("resize-layout.wav");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let initial_frame = runtime.frame_with_default_theme();
    let initial_extension_x = first_row_extension_x(&initial_frame);

    runtime.dispatch_message(crate::native_app::test_support::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::ResizeFileColumn(
            String::from("name"),
            radiant::widgets::DragHandleMessage::started(Point::new(0.0, 0.0)),
        ),
    ));
    runtime.dispatch_message(crate::native_app::test_support::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::FolderBrowserMessage::ResizeFileColumn(
            String::from("name"),
            radiant::widgets::DragHandleMessage::moved(Point::new(120.0, 0.0)),
        ),
    ));
    let resized_name_width = runtime
        .bridge()
        .state()
        .library
        .folder_browser
        .visible_file_columns()
        .into_iter()
        .find(|column| column.id == "name")
        .map(|column| column.width)
        .expect("name column should exist");
    assert!(resized_name_width >= 340.0, "{resized_name_width}");

    let resized_frame = runtime.frame_with_default_theme();
    let resized_extension_x = first_row_extension_x(&resized_frame);

    assert!(
        resized_extension_x >= initial_extension_x + 100.0,
        "resizing the name column should immediately move row cells: before={initial_extension_x}, after={resized_extension_x}"
    );
}

fn first_row_extension_x(frame: &SurfaceFrame) -> f32 {
    let texts = frame
        .paint_plan
        .text_runs()
        .map(|text| (text.text.as_str().to_string(), text.rect))
        .collect::<Vec<_>>();
    texts
        .iter()
        .filter(|(text, rect)| text.as_str() == "wav" && rect.min.y >= 336.0 && rect.min.y < 360.0)
        .map(|(_, rect)| rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("first row extension should paint: {texts:?}"))
}

#[test]
fn full_gui_column_drag_commits_on_release_and_clears_feedback() {
    let state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let rating_rect = frame
        .paint_plan
        .text_runs()
        .filter(|text| text.text.as_str() == "Rating")
        .map(|text| text.rect)
        .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .expect("rating column header should paint");
    let press = rating_rect.center();
    let drag = Point::new(press.x + 260.0, press.y);
    let drag_update = Point::new(drag.x + 1.0, drag.y);

    let press_target = runtime.dispatch_event(Event::primary_press(press));
    let move_target = runtime.dispatch_event(Event::pointer_move(drag));
    let update_target = runtime.dispatch_event(Event::pointer_move(drag_update));
    assert!(press_target.is_some());
    assert!(move_target.is_some());
    assert_eq!(update_target, press_target);
    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "rating",
            "collection",
            "extension",
            "size",
            "modified"
        ],
        "drag motion should preview without committing the column order"
    );

    let dragging_frame = runtime.frame_with_default_theme();
    assert!(dragging_frame.paint_plan.fill_rects().any(|fill| {
        fill.color == Rgba8::new(255, 160, 82, 230)
            && fill.rect.width() <= 2.5
            && fill.rect.height() >= 20.0
    }));
    assert!(dragging_frame.paint_plan.text_runs().any(|text| {
        text.text == "Rating"
            && text.rect.min.x >= drag_update.x
            && text.rect.min.y >= drag_update.y
    }));

    let release_target = runtime.dispatch_event(Event::primary_release(drag_update));
    assert_eq!(release_target, press_target);
    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .file_column_drag_feedback(),
        None
    );
    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "collection",
            "extension",
            "size",
            "rating",
            "modified"
        ]
    );

    let released_frame = runtime.frame_with_default_theme();
    assert!(!released_frame.paint_plan.fill_rects().any(|fill| {
        fill.color == Rgba8::new(255, 160, 82, 230)
            && fill.rect.width() <= 2.5
            && fill.rect.height() >= 20.0
    }));
    assert!(!released_frame.paint_plan.text_runs().any(|text| {
        text.text == "Rating"
            && text.rect.min.x >= drag_update.x
            && text.rect.min.y >= drag_update.y
    }));
}

#[test]
fn full_gui_column_drag_marker_uses_header_local_coordinates() {
    let state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rating_header_id = radiant::widgets::stable_widget_id(
        crate::native_app::ui::ids::SAMPLE_HEADER_SORT_DRAG_ID,
        "rating",
    );
    let modified_header_id = radiant::widgets::stable_widget_id(
        crate::native_app::ui::ids::SAMPLE_HEADER_SORT_DRAG_ID,
        "modified",
    );
    let rating_rect = *runtime
        .layout()
        .rects
        .get(&rating_header_id)
        .expect("rating column header hit target should be laid out");
    let modified_rect = *runtime
        .layout()
        .rects
        .get(&modified_header_id)
        .expect("modified column header hit target should be laid out");
    let press = rating_rect.center();
    let hover_modified_left = Point::new(modified_rect.min.x + 2.0, press.y);
    let hover_modified_left_update = Point::new(hover_modified_left.x + 1.0, hover_modified_left.y);

    runtime.dispatch_event(Event::primary_press(press));
    runtime.dispatch_event(Event::pointer_move(hover_modified_left));
    runtime.dispatch_event(Event::pointer_move(hover_modified_left_update));
    let dragging_frame = runtime.frame_with_default_theme();
    let marker = dragging_frame
        .paint_plan
        .fill_rects()
        .find(|fill| {
            fill.color == Rgba8::new(255, 160, 82, 230)
                && fill.rect.width() <= 2.5
                && fill.rect.height() >= 20.0
        })
        .expect("dragging over a later header should paint the drop marker");
    let handle_gap = marker.rect.min.x - modified_rect.min.x;
    assert!(
        (-18.0..=2.0).contains(&handle_gap),
        "drop marker should paint near the modified header's leading resize handle, marker={:?}, modified={modified_rect:?}, gap={handle_gap}",
        marker.rect
    );
}

#[test]
fn full_gui_sample_drag_back_to_list_clears_folder_drop_target_highlight() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    let loops = source_root.path().join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let sample = drums.join("kick.wav");
    fs::write(&sample, []).expect("write sample");
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ),
    );
    state
        .library
        .folder_browser
        .select_file(sample.display().to_string());

    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let initial_frame = runtime.frame_with_default_theme();
    let sample_press = text_center(&initial_frame, "kick");
    let sample_drag_start = Point::new(sample_press.x + 16.0, sample_press.y);
    let loops_target = text_center(&initial_frame, "loops");

    let press_target = runtime.dispatch_event(Event::primary_press(sample_press));
    let drag_start_target = runtime.dispatch_event(Event::pointer_move(sample_drag_start));
    runtime.dispatch_event(Event::pointer_move(loops_target));

    assert!(
        press_target.is_some(),
        "sample row should accept drag press"
    );
    assert!(
        drag_start_target.is_some(),
        "sample row should emit the drag start before folder hover"
    );
    let dragging_frame = runtime.frame_with_default_theme();
    assert!(
        dragging_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == FOLDER_DROP_TARGET_FILL),
        "active folder drop target should paint its background highlight"
    );

    runtime.dispatch_event(Event::pointer_move(sample_press));
    let returned_frame = runtime.frame_with_default_theme();
    assert!(
        !returned_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == FOLDER_DROP_TARGET_FILL),
        "moving back over the sample list should clear the folder drop target"
    );

    runtime.dispatch_event(Event::primary_release(sample_press));
    let released_frame = runtime.frame_with_default_theme();
    assert!(
        !runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .drag_active(),
        "dropping back on the sample list must cancel the browser drag"
    );
    assert_eq!(
        runtime.bridge().state().ui.status.sample,
        "Drag cancelled",
        "dropping back on the sample list should be reported as cancellation"
    );
    assert!(
        !released_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == FOLDER_DROP_TARGET_FILL),
        "dropping back on the sample list must not leave stale folder drop feedback"
    );
}

fn text_center(frame: &SurfaceFrame, label: &str) -> Point {
    frame
        .paint_plan
        .text_runs()
        .find(|text| text.text.as_str() == label)
        .map(|text| text.rect.center())
        .unwrap_or_else(|| panic!("{label} should paint"))
}

#[test]
fn sample_browser_rows_match_keyboard_scroll_stride() {
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::sample_browser(&mut state)
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
            ((pair[1] - pair[0]) - crate::native_app::test_support::SAMPLE_BROWSER_ROW_HEIGHT).abs()
                < 0.5
        }),
        "{row_tops:?}"
    );
}

#[test]
fn sample_browser_keyboard_scroll_keeps_two_context_rows() {
    assert_eq!(
        crate::native_app::test_support::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
        2
    );
    assert_eq!(
        crate::native_app::test_support::SAMPLE_BROWSER_ROW_HEIGHT,
        22.0
    );
}

#[test]
fn selected_sample_browser_row_paints_strong_fill_and_left_marker() {
    let widget = sample_hit_target(true, false, false, false);
    let bounds = Rect::from_xy_size(12.0, 8.0, 240.0, 22.0);
    let plan = widget.paint_plan_with_defaults(bounds);
    let fills = plan.fill_rects().collect::<Vec<_>>();

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
    let bounds = Rect::from_size(180.0, 22.0);
    let mut hit_target = sample_hit_target(false, false, false, false);

    assert_eq!(
        hit_target.handle_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0)),),
        None
    );

    let plan = hit_target.paint_plan_with_defaults(bounds);
    let fills = plan.fill_rects().collect::<Vec<_>>();

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
    let mut state = crate::native_app::test_support::NativeAppState::load_default()
        .expect("default state loads");
    let expected_names = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.stem.clone())
        .collect::<Vec<_>>();
    let frame = crate::native_app::test_support::view(&mut state)
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
