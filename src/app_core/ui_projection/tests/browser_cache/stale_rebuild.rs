use super::*;
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
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);
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
        sound_type: None,
        locked: false,
        missing: true,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);
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
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
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
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: Some(1),
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);
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

    let Some(cached) = project_cached_browser_row(&mut controller, 0, 1_000_000_000) else {
        panic!("cached row should exist");
    };

    assert_eq!(
        cached.0.playback_age_bucket,
        PlaybackAgeBucket::OlderThanMonth
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
                playback_age_bucket: PlaybackAgeBucket::Fresh,
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
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
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
