use super::*;
#[test]
/// Browser row projection should reuse the retained pipeline snapshot without faulting wav pages.
fn browser_rows_projection_uses_pipeline_snapshot_when_pages_are_unloaded() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
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
    controller.clear_loaded_wav_pages_for_tests();

    let mut rows = crate::app_core::actions::NativeRetainedVec::new();
    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label.as_ref(), "kick");
    assert!(controller.loaded_wav_pages_are_empty_for_tests());
}

#[test]
/// Browser row rendering should not queue feature refresh work during frame-time projection.
fn browser_rows_projection_does_not_queue_feature_cache_refresh() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    let source_id = crate::sample_sources::SourceId::new();
    controller.select_browser_source_for_tests(source_id);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
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
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_list(vec![0usize]);
    controller.clear_pending_browser_feature_cache_refresh_for_tests();

    let mut rows = crate::app_core::actions::NativeRetainedVec::new();
    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert!(!controller.has_pending_browser_feature_cache_refresh_for_tests());
}
