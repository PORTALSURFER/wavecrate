use super::super::*;

#[test]
/// Sidebar input edits should miss the dedicated sidebar segment without rebuilding frame chrome.
fn projection_segment_browser_tag_sidebar_refreshes_input_without_browser_frame_churn() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();

    controller.ui.browser.tag_sidebar_input = String::from("texture");

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_FRAME)
    );
    let lookup_counts = cache.take_segment_lookup_counts();
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 0, 1);
    assert_eq!(model.browser.tag_sidebar.input_value.as_str(), "texture");
}

#[test]
/// Sidebar metadata edits should invalidate the dedicated retained sidebar contract.
fn projection_segment_browser_tag_sidebar_refreshes_pills_after_metadata_edit() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.selection.last_focused_path = Some(PathBuf::from("kick.wav"));
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);

    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();
    assert_eq!(
        first_model.browser.tag_sidebar.exclusive_pills[0].state,
        crate::app_core::actions::NativeBrowserTagState::Off
    );

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop pill should update optimistically");

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::BROWSER_FRAME | NativeDirtySegments::BROWSER_ROWS_WINDOW
        )
    );
    let lookup_counts = cache.take_segment_lookup_counts();
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_eq!(
        second_model.browser.tag_sidebar.exclusive_pills[0].state,
        crate::app_core::actions::NativeBrowserTagState::On
    );
    assert_eq!(
        second_model.browser.tag_sidebar.exclusive_pills[1].state,
        crate::app_core::actions::NativeBrowserTagState::Off
    );
}

#[test]
/// Same-count sidebar target swaps should miss the sidebar segment without forcing frame churn.
fn projection_segment_browser_tag_sidebar_refreshes_for_same_count_selection_swap() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.tag_sidebar_open = true;
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: PathBuf::from("first.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-a")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
        crate::sample_sources::WavEntry {
            relative_path: PathBuf::from("second.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-b")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: true,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
    ]);
    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();

    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();
    assert_eq!(
        first_model.browser.tag_sidebar.header_label.as_str(),
        "first.wav"
    );
    assert_eq!(
        first_model.browser.tag_sidebar.exclusive_pills[0].state,
        crate::app_core::actions::NativeBrowserTagState::Off
    );

    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("second.wav")];
    controller.mark_browser_selected_paths_changed();

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::BROWSER_FRAME | NativeDirtySegments::BROWSER_ROWS_WINDOW
        )
    );
    let lookup_counts = cache.take_segment_lookup_counts();
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_eq!(
        second_model.browser.tag_sidebar.header_label.as_str(),
        "second.wav"
    );
    assert_eq!(
        second_model.browser.tag_sidebar.exclusive_pills[0].state,
        crate::app_core::actions::NativeBrowserTagState::On
    );
}
