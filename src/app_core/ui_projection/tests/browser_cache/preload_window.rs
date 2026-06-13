use super::*;
#[test]
/// Label lookup should fill from the retained browser pipeline when wav pages are absent.
fn label_lookup_uses_pipeline_snapshot_when_pages_are_unloaded() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("folder/snare.wav"),
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

    let label = controller.wav_label(0).expect("label should exist");

    assert_eq!(label, "snare");
    assert!(controller.loaded_wav_pages_are_empty_for_tests());
}

#[test]
/// BPM preload ranges should only include rows newly entering an unchanged browser window.
fn browser_bpm_preload_ranges_only_include_window_delta() {
    let source_id = crate::sample_sources::SourceId::new();
    let previous = ProjectedBrowserPreloadWindow {
        source_id: Some(source_id.clone()),
        visible_rows_revision: 11,
        window_start: 10,
        window_len: 5,
    };

    let ranges = browser_bpm_preload_ranges(Some(&previous), Some(&source_id), 11, 12, 5);

    assert_eq!(ranges, vec![(15, 2)]);
}
