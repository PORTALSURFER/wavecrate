use super::*;

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
    fs::write(loops.join("loop.wav"), []).expect("write loop sample");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
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
    assert!(
        press_target.is_some(),
        "sample row should accept drag press"
    );
    assert!(
        drag_start_target.is_some(),
        "sample row should emit the drag start before folder hover"
    );
    let dragging_list_frame = runtime.frame_with_default_theme();
    let dragging_list_texts = dragging_list_frame.paint_plan.text_label_strings();
    assert!(
        dragging_list_texts.iter().any(|text| text == "kick"),
        "starting a sample drag must not remove the active sample list rows: {dragging_list_texts:?}"
    );

    runtime.dispatch_event(Event::pointer_move(loops_target));
    let dragging_frame = runtime.frame_with_default_theme();
    let dragging_texts = dragging_frame.paint_plan.text_label_strings();
    assert!(
        dragging_texts.iter().any(|text| text == "kick"),
        "sample list rows must remain painted while hovering a folder drop target: {dragging_texts:?}"
    );
    assert!(
        dragging_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == folder_drop_target_fill()),
        "active folder drop target should paint its background highlight"
    );

    runtime.dispatch_event(Event::pointer_move(sample_press));
    let returned_frame = runtime.frame_with_default_theme();
    assert!(
        !returned_frame
            .paint_plan
            .fill_rects()
            .any(|fill| fill.color == folder_drop_target_fill()),
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
            .any(|fill| fill.color == folder_drop_target_fill()),
        "dropping back on the sample list must not leave stale folder drop feedback"
    );
}
