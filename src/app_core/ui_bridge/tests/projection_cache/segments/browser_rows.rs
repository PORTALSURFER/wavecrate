use super::super::*;

#[test]
fn projection_segment_browser_rows_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.mark_browser_selected_paths_changed();
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_ROWS_WINDOW)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

#[test]
fn projection_segment_auto_rename_progress_updates_only_browser_rows() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
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
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_all(1);
    controller
        .begin_auto_rename_batch_for_tests(source.id.clone(), vec![PathBuf::from("kick.wav")]);

    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();

    controller.apply_auto_rename_progress_for_tests(
        crate::app::controller::jobs::SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("kick.wav"),
        },
    );

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    let lookup_counts = cache.take_segment_lookup_counts();
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_ROWS_WINDOW)
    );
    assert_eq!(
        model.browser.rows[0].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Active
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}
