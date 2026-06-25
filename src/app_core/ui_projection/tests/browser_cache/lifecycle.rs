use super::*;
#[test]
/// Retained browser row cache should survive visible-row revision changes for the same source.
fn browser_row_cache_persists_when_visible_revision_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    let source_id = crate::sample_sources::SourceId::new();
    controller.select_browser_source_for_tests(source_id.clone());
    controller.projected_browser_rows_source_id = Some(source_id);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: PlaybackAgeBucket::Fresh,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            locked: false,
            marked: false,
            bpm_value_bits: None,
            long_sample_mark: false,
            last_used_tick: 1,
        },
    );
    controller.ui.browser.viewport.visible_rows_revision = 8;

    refresh_projected_browser_row_cache(&mut controller);

    assert!(controller.projected_browser_rows.contains_key(&0));
}

#[test]
/// Retained browser row cache should clear when the selected source changes.
fn browser_row_cache_clears_when_selected_source_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.projected_browser_rows_source_id = Some(crate::sample_sources::SourceId::new());
    controller.select_browser_source_for_tests(crate::sample_sources::SourceId::new());
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: PlaybackAgeBucket::Fresh,
            bucket_label: String::new(),
            missing: false,
            looped: false,
            locked: false,
            marked: false,
            bpm_value_bits: None,
            long_sample_mark: false,
            last_used_tick: 1,
        },
    );

    refresh_projected_browser_row_cache(&mut controller);

    assert!(controller.projected_browser_rows.is_empty());
}

#[test]
/// Selected-path lookup cache should refresh when path content changes at equal length.
fn selected_path_lookup_refreshes_for_same_len_path_changes() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("first.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-a")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            last_curated_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("second.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-b")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            last_curated_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
    ]);
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();
    refresh_projected_selected_paths_lookup(&mut controller);
    assert!(matches!(
        controller.projected_selected_paths_lookup,
        Some(crate::app_core::controller::ProjectedSelectedPathsLookup::Single(0))
    ));
    assert!(selected_index_is_selected(&controller, 0));
    assert!(!selected_index_is_selected(&controller, 1));

    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("second.wav")];
    controller.mark_browser_selected_paths_changed();
    refresh_projected_selected_paths_lookup(&mut controller);
    assert!(matches!(
        controller.projected_selected_paths_lookup,
        Some(crate::app_core::controller::ProjectedSelectedPathsLookup::Single(1))
    ));
    assert!(!selected_index_is_selected(&controller, 0));
    assert!(selected_index_is_selected(&controller, 1));
}
