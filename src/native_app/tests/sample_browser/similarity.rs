use super::*;

#[test]
fn sample_row_selection_still_works_in_similarity_mode() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    let near = drums.join("near.wav");
    fs::write(&anchor, []).expect("write anchor");
    fs::write(&near, []).expect("write near");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ),
    );
    let anchor_id = anchor.display().to_string();
    let near_id = near.display().to_string();
    state
        .library
        .folder_browser
        .set_similarity_scores_for_tests(anchor_id, [(near_id.clone(), 0.9)].into_iter().collect());
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();

    runtime.dispatch_primary_click(text_center(&frame, "near"));

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(near_id.as_str())
    );
}

#[test]
fn sample_browser_renders_similarity_header_only_in_similarity_mode() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    fs::write(&anchor, []).expect("write anchor");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
        ),
    );

    let inactive_frame =
        crate::native_app::test_support::sample_browser::sample_browser(&mut state)
            .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    assert!(!inactive_frame.paint_plan.contains_text("Sim"));

    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            anchor.display().to_string(),
        ),
    );
    let active_frame = crate::native_app::test_support::sample_browser::sample_browser(&mut state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    assert!(active_frame.paint_plan.contains_text("Sim"));
}
