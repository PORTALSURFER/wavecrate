use super::*;
use crate::native_app::tests::NativeRuntimeForTests;

fn sample_header_center(frame: &SurfaceFrame, label_prefix: &str) -> Point {
    frame
        .paint_plan
        .text_runs()
        .filter(|text| text.text.as_str().starts_with(label_prefix))
        .map(|text| text.rect)
        .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .map(|rect| rect.center())
        .unwrap_or_else(|| panic!("{label_prefix} column header should paint"))
}

fn rendered_sample_stems(frame: &SurfaceFrame, stems: &[&str]) -> Vec<String> {
    let mut rows = frame
        .paint_plan
        .text_runs()
        .filter_map(|text| {
            stems
                .iter()
                .find(|stem| text.text.as_str() == **stem)
                .map(|stem| ((*stem).to_owned(), text.rect.min.y))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|(_, left_y), (_, right_y)| left_y.total_cmp(right_y));
    rows.into_iter().map(|(stem, _)| stem).collect()
}

fn primary_click_with_jitter(runtime: &mut NativeRuntimeForTests, point: Point) {
    let jitter = Point::new(point.x + 1.0, point.y + 1.0);
    assert!(
        runtime
            .dispatch_event(Event::primary_press(point))
            .is_some()
    );
    assert!(
        runtime
            .dispatch_event(Event::pointer_move(jitter))
            .is_some()
    );
    assert!(
        runtime
            .dispatch_event(Event::primary_release(jitter))
            .is_some()
    );
}

#[test]
fn sample_browser_frame_paints_column_and_file_text() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("column-header.wav");
    let expected_stem = std::path::Path::new(&selected_file)
        .file_stem()
        .expect("temp sample has a file stem")
        .to_string_lossy();
    prepare_sample_browser_view(&mut state);
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    let texts = frame.paint_plan.text_label_strings();

    assert!(
        texts.iter().any(|text| text.starts_with("Name")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with(expected_stem.as_ref())),
        "{texts:?}"
    );
}

#[test]
fn full_gui_column_header_click_toggles_sort_direction() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    for (name, bytes) in [
        ("alpha.wav", vec![0_u8; 10]),
        ("beta.wav", vec![0_u8; 20]),
        ("gamma.wav", vec![0_u8; 5]),
    ] {
        fs::write(source_root.path().join(name), bytes).expect("sample file");
    }
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(1300.0, 620.0));
    let frame = runtime.frame_with_default_theme();
    let stems = ["alpha", "beta", "gamma"];

    assert_eq!(rendered_sample_stems(&frame, &stems), stems);

    primary_click_with_jitter(&mut runtime, sample_header_center(&frame, "Name"));
    assert_eq!(
        rendered_sample_stems(&runtime.frame_with_default_theme(), &stems),
        ["gamma", "beta", "alpha"]
    );

    let size_ascending_frame = runtime.frame_with_default_theme();
    primary_click_with_jitter(
        &mut runtime,
        sample_header_center(&size_ascending_frame, "Size"),
    );
    let sort = runtime.bridge().state().library.folder_browser.file_sort();
    assert_eq!(sort.column_id, "size");
    assert_eq!(
        sort.direction,
        radiant::application::SortDirection::Ascending
    );
    assert_eq!(
        rendered_sample_stems(&runtime.frame_with_default_theme(), &stems),
        ["gamma", "alpha", "beta"]
    );

    let size_descending_frame = runtime.frame_with_default_theme();
    primary_click_with_jitter(
        &mut runtime,
        sample_header_center(&size_descending_frame, "Size"),
    );
    let sort = runtime.bridge().state().library.folder_browser.file_sort();
    assert_eq!(sort.column_id, "size");
    assert_eq!(
        sort.direction,
        radiant::application::SortDirection::Descending
    );
    assert_eq!(
        rendered_sample_stems(&runtime.frame_with_default_theme(), &stems),
        ["beta", "alpha", "gamma"]
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

    prepare_sample_browser_view(&mut state);
    let frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
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
    let name_center = sample_header_center(&frame, "Name");
    let point = Point::new(name_center.x - 8.0, name_center.y);
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
