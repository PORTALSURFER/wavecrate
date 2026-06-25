use super::super::*;

/// Non-segment static-key changes should only set the global static dirty bit.
#[test]
fn projection_segment_non_segment_static_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.volume = 0.75;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::GLOBAL_STATIC)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Prompt/progress/drag app-key misses should route through the overlay dirty lane.
#[test]
fn projection_segment_overlay_only_changes_keep_segment_hits_and_static_clean() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.progress.visible = true;
        controller.ui.progress.modal = true;
        controller.ui.progress.completed = 2;
        controller.ui.progress.total = 5;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Overlay-only misses should preserve retained static fields while refreshing overlays.
#[test]
fn projection_overlay_only_miss_skips_static_non_segment_refresh() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let mut retained = Arc::unwrap_or_clone(first_model);
    retained.sources_label = String::from("sentinel");
    cache.app_model = Some(Arc::new(retained));

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 1;
    controller.ui.progress.total = 3;

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert_eq!(model.sources_label.as_str(), "sentinel");
    assert!(model.progress_overlay.visible);
}

#[test]
fn projection_overlay_only_miss_refreshes_options_panel_fields() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let mut retained = Arc::unwrap_or_clone(first_model);
    retained.options_panel.visible = false;
    retained.options_panel.trash_folder_label = None;
    cache.app_model = Some(Arc::new(retained));

    controller.ui.options_panel.open = true;
    controller.ui.trash_folder = Some(PathBuf::from("trash_bin"));

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert!(model.options_panel.visible);
    assert_eq!(
        model.options_panel.trash_folder_label.as_deref(),
        Some("trash_bin")
    );
}

#[test]
fn projection_overlay_only_miss_reuses_unique_snapshot_arc() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let first_ptr = Arc::as_ptr(&first_model);
    drop(first_model);

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 1;
    controller.ui.progress.total = 3;

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);

    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert_eq!(Arc::as_ptr(&second_model), first_ptr);
    assert!(second_model.progress_overlay.visible);
}

#[test]
fn projection_audio_engine_dirty_refreshes_retained_chip_and_panel_state() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.audio.applied = Some(projection_fixtures::active_audio_output(
        "asio",
        "Studio",
        48_000,
        Some(256),
        2,
    ));
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let mut retained = Arc::unwrap_or_clone(first_model);
    retained.audio_engine.chip_label = String::from("sentinel");
    retained.audio_engine.detail_label = None;
    cache.app_model = Some(Arc::new(retained));

    controller.ui.audio.output_runtime_error = Some(String::from("USB disconnected"));
    controller.ui.options_panel.open = true;
    controller.ui.options_panel.active_audio_picker = Some(AudioPickerTarget::OutputHost);

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::GLOBAL_STATIC | NativeDirtySegments::STATE_OVERLAY
        )
    );
    assert_eq!(model.audio_engine.chip_label, "Audio Err");
    assert_eq!(
        model.audio_engine.detail_label.as_deref(),
        Some("USB disconnected")
    );
    assert_eq!(
        model.audio_engine.active_picker,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost)
    );
}

#[test]
fn projection_overlay_only_miss_clones_when_prior_snapshot_is_aliased() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    assert!(!first_model.progress_overlay.visible);

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 2;
    controller.ui.progress.total = 5;

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);

    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert!(!Arc::ptr_eq(&first_model, &second_model));
    assert!(!first_model.progress_overlay.visible);
    assert!(second_model.progress_overlay.visible);
}

/// Overlay-only misses should retain browser row text storage when the prior snapshot is aliased.
#[test]
fn projection_overlay_only_miss_reuses_browser_row_text_arcs() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
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
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_list(vec![0usize]);

    let mut cache = UiProjectionCache::default();
    let (first_model, _) = cache.resolve_or_project(&mut controller);
    let first_row = &first_model.browser.rows[0];
    let first_label = Arc::clone(&first_row.label);

    controller.ui.progress.visible = true;
    controller.ui.progress.modal = true;
    controller.ui.progress.completed = 1;
    controller.ui.progress.total = 3;

    let (second_model, dirty_segments) = cache.resolve_or_project(&mut controller);
    let second_row = &second_model.browser.rows[0];

    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATE_OVERLAY)
    );
    assert!(!Arc::ptr_eq(&first_model, &second_model));
    assert!(Arc::ptr_eq(&first_label, &second_row.label));
}
