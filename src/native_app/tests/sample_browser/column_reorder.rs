use super::*;

#[test]
fn sample_column_resize_updates_rendered_row_layout_without_sorting() {
    let (state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("resize-layout.wav");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(1300.0, 620.0));
    let initial_frame = runtime.frame_with_default_theme();
    let initial_extension_x = first_row_extension_x(&initial_frame, "resize-layout");

    runtime.dispatch_message(crate::native_app::test_support::state::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::ResizeFileColumn(
            String::from("name"),
            radiant::widgets::DragHandleMessage::started(Point::new(0.0, 0.0)),
        ),
    ));
    runtime.dispatch_message(crate::native_app::test_support::state::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::ResizeFileColumn(
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
    let resized_extension_x = first_row_extension_x(&resized_frame, "resize-layout");

    assert!(
        resized_extension_x >= initial_extension_x + 100.0,
        "resizing the name column should immediately move row cells: before={initial_extension_x}, after={resized_extension_x}"
    );
}

fn first_row_extension_x(frame: &SurfaceFrame, sample_stem: &str) -> f32 {
    let texts = frame
        .paint_plan
        .text_runs()
        .map(|text| (text.text.as_str().to_string(), text.rect))
        .collect::<Vec<_>>();
    let stem_rect = texts
        .iter()
        .find(|(text, _)| text == sample_stem)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("first row sample stem should paint: {texts:?}"));
    texts
        .iter()
        .filter(|(text, rect)| {
            text.as_str() == "wav"
                && rect.min.y >= stem_rect.min.y - 1.0
                && rect.min.y <= stem_rect.max.y + 1.0
        })
        .map(|(_, rect)| rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("first row extension should paint: {texts:?}"))
}

#[test]
fn full_gui_column_drag_commits_on_release_and_clears_feedback() {
    let state = crate::native_app::test_support::state::NativeAppState::load_default()
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
            "playback_type",
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
            "playback_type",
            "collection",
            "extension",
            "rating",
            "size",
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
    let state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(1100.0, 620.0));
    let rating_header_id = radiant::application::compact_details_header_sort_drag_id(
        radiant::widgets::stable_widget_id(
            crate::native_app::ui::ids::RETAINED_SAMPLE_HEADER_CELL_ID,
            "rating",
        ),
    );
    let size_header_id = radiant::application::compact_details_header_sort_drag_id(
        radiant::widgets::stable_widget_id(
            crate::native_app::ui::ids::RETAINED_SAMPLE_HEADER_CELL_ID,
            "size",
        ),
    );
    let extension_resize_id =
        radiant::application::compact_details_header_resize_id(radiant::widgets::stable_widget_id(
            crate::native_app::ui::ids::RETAINED_SAMPLE_HEADER_CELL_ID,
            "extension",
        ));
    let rating_rect = *runtime
        .layout()
        .rects
        .get(&rating_header_id)
        .expect("rating column header hit target should be laid out");
    let size_rect = *runtime
        .layout()
        .rects
        .get(&size_header_id)
        .expect("size column header hit target should be laid out");
    let extension_resize_rect = *runtime
        .layout()
        .rects
        .get(&extension_resize_id)
        .expect("extension column divider should be laid out");
    let press = rating_rect.center();
    let hover_size_left = Point::new(size_rect.min.x + 12.0, press.y);
    let hover_size_left_update = Point::new(hover_size_left.x + 1.0, hover_size_left.y);

    runtime.dispatch_event(Event::primary_press(press));
    runtime.dispatch_event(Event::pointer_move(hover_size_left));
    runtime.dispatch_event(Event::pointer_move(hover_size_left_update));
    let dragging_frame = runtime.frame_with_default_theme();
    let marker = column_drop_marker_rect(&dragging_frame);
    let divider_delta = marker.center().x - extension_resize_rect.center().x;
    assert!(
        divider_delta.abs() <= 2.0,
        "drop marker should paint on the dotted divider before the size header, marker={marker:?}, divider={extension_resize_rect:?}, delta={divider_delta}",
    );
}

fn column_drop_marker_rect(frame: &SurfaceFrame) -> Rect {
    frame
        .paint_plan
        .fill_rects()
        .find(|fill| {
            fill.color == Rgba8::new(255, 160, 82, 230)
                && fill.rect.width() <= 2.5
                && fill.rect.height() >= 20.0
        })
        .map(|fill| fill.rect)
        .expect("active column drag should paint the drop marker")
}
