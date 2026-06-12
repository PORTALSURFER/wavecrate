use super::*;

#[test]
fn sample_browser_frame_paints_column_and_file_text() {
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let expected_stem = state
        .library
        .folder_browser
        .selected_audio_files()
        .first()
        .map(|file| file.stem.clone())
        .expect("default assets include an audio sample");
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&mut state)
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
    let mut state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(292.0, 8.0)),
        ),
    );
    state.library.folder_browser.apply_message(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(420.0, 8.0)),
        ),
    );

    let frame = crate::native_app::test_support::sample_browser::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));

    assert!(frame.paint_plan.fill_rects().any(|fill| {
        fill.color == Rgba8::new(255, 160, 82, 230)
            && fill.rect.width() <= 2.5
            && fill.rect.height() >= 20.0
    }));
}

#[test]
fn sample_browser_header_paints_hover_affordance() {
    let state = crate::native_app::test_support::state::NativeAppState::load_default()
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
    let state = crate::native_app::test_support::state::NativeAppState::load_default()
        .expect("default state loads");
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));

    runtime.dispatch_message(crate::native_app::test_support::state::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::started(Point::new(600.0, 320.0)),
        ),
    ));
    runtime.dispatch_message(crate::native_app::test_support::state::GuiMessage::FolderBrowser(
        crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage::DragFileColumn(
            String::from("rating"),
            radiant::widgets::DragHandleMessage::moved(Point::new(620.0, 320.0)),
        ),
    ));

    let frame = runtime.frame_with_default_theme();

    assert!(frame.paint_plan.text_runs().any(|text| {
        text.text == "Rating" && text.rect.min.x >= 620.0 && text.rect.min.y >= 330.0
    }));
}
