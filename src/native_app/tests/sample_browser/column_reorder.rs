use super::*;

#[test]
fn sample_column_resize_updates_rendered_row_layout_without_sorting() {
    let (state, _source_root, _selected_file) =
        native_app_state_with_temp_sample("resize-layout.wav");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let initial_frame = runtime.frame_with_default_theme();
    let initial_extension_x = first_row_extension_x(&initial_frame);

    runtime.dispatch_message(crate::native_app::test_support::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::ResizeFileColumn(
            String::from("name"),
            radiant::widgets::DragHandleMessage::started(Point::new(0.0, 0.0)),
        ),
    ));
    runtime.dispatch_message(crate::native_app::test_support::GuiMessage::FolderBrowser(
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
        (-42.0..=2.0).contains(&handle_gap),
        "drop marker should paint near the modified header's leading resize handle, marker={:?}, modified={modified_rect:?}, gap={handle_gap}",
        marker.rect
    );
}
