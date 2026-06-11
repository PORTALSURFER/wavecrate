use super::*;

/// The retained projection registry is the source of truth for segment dirty masks.
#[test]
fn retained_segment_handler_plan_matches_dirty_segment_contract() {
    assert_eq!(
        retained_segment_handler_plan(),
        vec![
            (
                ProjectionSegment::StatusBar,
                NativeDirtySegments::STATUS_BAR
            ),
            (
                ProjectionSegment::BrowserFrame,
                NativeDirtySegments::BROWSER_FRAME
            ),
            (
                ProjectionSegment::BrowserTagSidebar,
                NativeDirtySegments::BROWSER_FRAME
            ),
            (
                ProjectionSegment::BrowserRowsWindow,
                NativeDirtySegments::BROWSER_ROWS_WINDOW
            ),
            (ProjectionSegment::MapPanel, NativeDirtySegments::MAP_PANEL),
            (
                ProjectionSegment::WaveformOverlay,
                NativeDirtySegments::WAVEFORM_OVERLAY
            ),
        ]
    );
}

/// No-op pulls should report all retained segment hits and no dirty mask bits.
#[test]
fn projection_segment_noop_pull_hits_all_segments() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|_| {});
    assert_eq!(dirty_segments, NativeDirtySegments::empty());
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Status-key changes should rematerialize only the status segment.
#[test]
fn projection_segment_status_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.projection_revisions.status =
            controller.ui.projection_revisions.status.wrapping_add(1);
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::STATUS_BAR)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Browser-frame changes should stay isolated from browser-row window materialization.
#[test]
fn projection_segment_browser_frame_dirty_mask_and_lookup_counts() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.browser.search.sort = SampleBrowserSort::PlaybackAgeAsc;
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(NativeDirtySegments::BROWSER_FRAME)
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Retained browser-frame materialization must copy active rating-filter flags.
#[test]
fn projection_segment_browser_frame_copies_active_rating_filters() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.ui.browser.search.rating_filter.insert(3);
    controller.ui.browser.search.rating_filter.insert(4);
    controller.mark_browser_search_projection_revision_dirty();

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::BROWSER_FRAME
        )
    );
    assert!(model.browser.active_rating_filters[6]);
    assert_eq!(
        model.browser.active_rating_filters,
        [false, false, false, false, false, false, true, true]
    );
}

/// Retained browser-frame materialization must copy active playback-age filter flags.
#[test]
fn projection_segment_browser_frame_copies_active_playback_age_filters() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(crate::app::state::PlaybackAgeFilterChip::NeverPlayed);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(crate::app::state::PlaybackAgeFilterChip::OlderThanWeek);
    controller.mark_browser_search_projection_revision_dirty();

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::BROWSER_FRAME
        )
    );
    assert_eq!(
        model.browser.active_playback_age_filters,
        [true, false, true]
    );
}

#[test]
fn projection_segment_browser_frame_copies_marked_filter_state() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);

    controller.ui.browser.search.marked_only = true;
    controller.mark_browser_search_projection_revision_dirty();

    let (model, dirty_segments) = cache.resolve_or_project(&mut controller);
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::BROWSER_FRAME
        )
    );
    assert!(model.browser.marked_filter_active);
}

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

/// Anchor-only browser updates should stay on the browser frame segment.
#[test]
fn projection_segment_browser_anchor_change_skips_browser_rows_window() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.browser.selection.selection_anchor_visible = Some(3);
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR | NativeDirtySegments::BROWSER_FRAME
        )
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 1, 0);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}

/// Focus-only browser updates should patch row state without rebuilding row content.
#[test]
fn projection_segment_browser_focus_change_updates_frame_and_rows() {
    let (dirty_segments, lookup_counts) = project_after_warm_cache(|controller| {
        controller.ui.browser.selection.selected_visible = Some(2);
    });
    assert_eq!(
        dirty_segments,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::BROWSER_FRAME | NativeDirtySegments::BROWSER_ROWS_WINDOW
        )
    );
    assert_segment_lookup_counts(lookup_counts.status_bar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_frame, 0, 1);
    assert_segment_lookup_counts(lookup_counts.browser_tag_sidebar, 1, 0);
    assert_segment_lookup_counts(lookup_counts.browser_rows_window, 0, 1);
    assert_segment_lookup_counts(lookup_counts.map_panel, 1, 0);
    assert_segment_lookup_counts(lookup_counts.waveform_overlay, 1, 0);
}
