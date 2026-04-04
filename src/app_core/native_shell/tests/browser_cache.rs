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
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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
            locked: false,
            missing: false,
            last_played_at: None,
        },
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("second.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("hash-b")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
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

#[test]
/// Cached browser rows should rebuild when stored tag/column metadata is stale.
fn cached_browser_row_rebuilds_when_stored_tag_column_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::KEEP_1,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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

    let Some(cached) = project_cached_browser_row(&mut controller, 0, 0) else {
        panic!("cached row should exist");
    };

    assert_eq!(cached.0.column_index, 2);
    assert_eq!(cached.0.rating_level, 1);
}

#[test]
/// Cached browser rows should rebuild when stored missing metadata is stale.
fn cached_browser_row_rebuilds_when_stored_missing_state_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: true,
        last_played_at: None,
    }]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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

    let Some(cached) = project_cached_browser_row(&mut controller, 0, 0) else {
        panic!("cached row should exist");
    };

    assert!(cached.0.missing);
}

#[test]
/// Cached browser rows should rebuild when stored marked metadata is stale.
fn cached_browser_row_rebuilds_when_stored_mark_state_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    let source_id = crate::sample_sources::SourceId::new();
    controller.select_browser_source_for_tests(source_id.clone());
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller
        .ui
        .browser
        .marks
        .toggle_paths(&source_id, &[std::path::PathBuf::from("kick.wav")]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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

    let Some(cached) = project_cached_browser_row(&mut controller, 0, 0) else {
        panic!("cached row should exist");
    };

    assert!(cached.0.marked);
}

#[test]
/// Cached browser rows should rebuild when stored playback-age metadata is stale.
fn cached_browser_row_rebuilds_when_stored_playback_age_bucket_is_stale() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("kick.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: Some(1),
    }]);
    controller.projected_browser_rows.insert(
        0,
        ProjectedBrowserRowCacheEntry {
            row_identity_hash: browser_row_identity_hash(std::path::Path::new("kick.wav")),
            relative_path: std::path::PathBuf::from("kick.wav"),
            row_label: String::from("Kick"),
            column_index: 1,
            rating_level: 0,
            playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
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

    let Some(cached) = project_cached_browser_row(&mut controller, 0, 1_000_000_000) else {
        panic!("cached row should exist");
    };

    assert_eq!(
        cached.0.playback_age_bucket,
        crate::app::state::PlaybackAgeBucket::OlderThanMonth
    );
}

#[test]
/// Retained browser-row cache should evict one least-recently-used row instead of clearing all rows.
fn browser_row_cache_evicts_one_lru_entry_at_capacity() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.projected_browser_rows_source_id = Some(crate::sample_sources::SourceId::new());
    for index in 0..MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE {
        let path = std::path::PathBuf::from(format!("cached-{index}.wav"));
        controller.projected_browser_rows.insert(
            index,
            ProjectedBrowserRowCacheEntry {
                row_identity_hash: browser_row_identity_hash(path.as_path()),
                relative_path: path,
                row_label: format!("Cached {index}"),
                column_index: 1,
                rating_level: 0,
                playback_age_bucket: crate::app::state::PlaybackAgeBucket::Fresh,
                bucket_label: String::new(),
                missing: false,
                looped: false,
                locked: false,
                marked: false,
                bpm_value_bits: None,
                long_sample_mark: false,
                last_used_tick: index as u64 + 1,
            },
        );
    }
    controller.projected_browser_row_cache_clock = MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE as u64;
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("fresh.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);

    let cached_path = {
        let Some((cached, _)) = project_cached_browser_row(&mut controller, 0, 0) else {
            panic!("cached row should exist");
        };
        cached.relative_path.clone()
    };

    assert_eq!(
        controller.projected_browser_rows.len(),
        MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE
    );
    assert_eq!(cached_path, std::path::PathBuf::from("fresh.wav"));
    assert!(controller.projected_browser_rows.contains_key(&1));
}

#[test]
/// Reusing the projection buffer should preserve the existing allocation.
fn browser_rows_projection_reuses_provided_buffer_capacity() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("snare.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::List(vec![0usize].into());
    let mut rows = Vec::new();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);
    let first_capacity = rows.capacity();
    let first_ptr = rows.as_ptr();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows.capacity(), first_capacity);
    assert_eq!(rows.as_ptr(), first_ptr);
}

#[test]
/// Row-state patching should update focused and selected flags without touching labels.
fn browser_rows_state_patch_updates_flags_without_rebuilding_labels() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("kick.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("kick-hash")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        },
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("snare.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("snare-hash")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        },
    ]);
    controller.ui.browser.viewport.visible =
        crate::app_core::app_api::state::VisibleRows::List(vec![0usize, 1usize].into());
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("snare.wav")];
    controller.mark_browser_selected_paths_changed();

    let mut rows = vec![
        crate::app_core::actions::NativeBrowserRowModel::new(0, "Kick", 1, true, true),
        crate::app_core::actions::NativeBrowserRowModel::new(1, "Snare", 1, false, false),
    ];

    patch_browser_rows_state(&mut controller, Some(1), &mut rows);

    assert_eq!(rows[0].label, "Kick");
    assert_eq!(rows[1].label, "Snare");
    assert!(!rows[0].selected);
    assert!(!rows[0].focused);
    assert!(rows[1].selected);
    assert!(rows[1].focused);
}

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
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.clear_loaded_wav_pages_for_tests();

    let mut rows = Vec::new();
    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "kick");
    assert!(controller.loaded_wav_pages_are_empty_for_tests());
}

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
        locked: false,
        missing: false,
        last_played_at: None,
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
