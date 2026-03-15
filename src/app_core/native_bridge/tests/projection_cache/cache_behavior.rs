use super::*;

#[test]
fn projection_cache_key_changes_when_browser_view_window_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.selection.autoscroll = false;
    controller.ui.browser.viewport.view_window_start = 7;
    controller.ui.browser.viewport.render_window_start = 7;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Projection cache key should change when normalized volume rounds to a new milli bucket.
fn projection_cache_key_changes_when_volume_milli_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.volume = 0.2001;
    let first = build_projection_cache_key(&controller);
    controller.ui.volume = 0.2009;
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
/// Full and segment waveform keys must keep static waveform milli conversion aligned.
fn projection_and_waveform_keys_share_waveform_milli_conversion() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.8, 0.2));
    controller.ui.waveform.edit_selection = Some(
        crate::selection::SelectionRange::new(0.7, 0.4)
            .with_fade_in(0.2, 0.8)
            .with_fade_in_mute(0.1)
            .with_fade_out(0.3, 0.2)
            .with_fade_out_mute(0.2),
    );
    controller.ui.waveform.view.start = 0.1;
    controller.ui.waveform.view.end = 0.9;

    let full = build_projection_cache_key(&controller);
    let segment = build_waveform_projection_key(&controller);
    assert_eq!(
        full.waveform_selection_start_milli,
        segment.waveform_selection_start_milli
    );
    assert_eq!(
        full.waveform_selection_end_milli,
        segment.waveform_selection_end_milli
    );
    assert_eq!(
        full.waveform_edit_selection_start_milli,
        segment.waveform_edit_selection_start_milli
    );
    assert_eq!(
        full.waveform_edit_selection_end_milli,
        segment.waveform_edit_selection_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_end_milli,
        segment.waveform_edit_fade_in_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_mute_start_milli,
        segment.waveform_edit_fade_in_mute_start_milli
    );
    assert_eq!(
        full.waveform_edit_fade_in_curve_milli,
        segment.waveform_edit_fade_in_curve_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_start_milli,
        segment.waveform_edit_fade_out_start_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_mute_end_milli,
        segment.waveform_edit_fade_out_mute_end_milli
    );
    assert_eq!(
        full.waveform_edit_fade_out_curve_milli,
        segment.waveform_edit_fade_out_curve_milli
    );
    assert_eq!(
        full.waveform_view_start_milli,
        segment.waveform_view_start_milli
    );
    assert_eq!(
        full.waveform_view_end_milli,
        segment.waveform_view_end_milli
    );
}

#[test]
/// Cursor/playhead motion should not invalidate static projection keys.
fn projection_and_waveform_keys_ignore_cursor_and_playhead_motion() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first_full = build_projection_cache_key(&controller);
    let first_waveform = build_waveform_projection_key(&controller);

    controller.ui.waveform.cursor = Some(0.1234);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.4321;

    let second_full = build_projection_cache_key(&controller);
    let second_waveform = build_waveform_projection_key(&controller);
    assert_eq!(first_full, second_full);
    assert_eq!(first_waveform, second_waveform);
}

#[test]
/// Waveform key should change when normalized view-range scalars round to new milli values.
fn waveform_projection_key_changes_when_view_milli_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.view.start = 0.1001;
    controller.ui.waveform.view.end = 0.8001;
    let first = build_waveform_projection_key(&controller);

    controller.ui.waveform.view.start = 0.1009;
    controller.ui.waveform.view.end = 0.8009;
    let second = build_waveform_projection_key(&controller);

    assert_ne!(first, second);
}

#[test]
/// Waveform option toggles must invalidate both full and waveform segment projection keys.
fn waveform_option_toggles_change_projection_and_waveform_keys() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let first_full = build_projection_cache_key(&controller);
    let first_waveform = build_waveform_projection_key(&controller);

    controller.ui.waveform.channel_view = crate::waveform::WaveformChannelView::SplitStereo;
    controller.ui.waveform.normalized_audition_enabled = true;
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.transient_snap_enabled = true;
    controller.ui.waveform.transient_markers_enabled = false;
    controller.ui.waveform.slice_mode_enabled = true;

    let second_full = build_projection_cache_key(&controller);
    let second_waveform = build_waveform_projection_key(&controller);
    assert_ne!(first_full, second_full);
    assert_ne!(first_waveform, second_waveform);
}

#[test]
/// Projection cache keys must change when selected-path revisions change.
fn projection_cache_key_changes_when_selected_path_revision_changes() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("first.wav")];
    controller.mark_browser_selected_paths_changed();
    let first = build_projection_cache_key(&controller);
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("second.wav")];
    controller.mark_browser_selected_paths_changed();
    let second = build_projection_cache_key(&controller);
    assert_ne!(first, second);
}

#[test]
fn projection_cache_reuses_model_when_key_unchanged() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let first = cache.resolve_or_project(&mut controller);
    let second = cache.resolve_or_project(&mut controller);
    assert!(Arc::ptr_eq(&first.0, &second.0));
    assert_eq!(second.1, NativeDirtySegments::empty());

    controller.set_status("changed", StatusTone::Info);
    let refreshed = cache.resolve_or_project(&mut controller);
    assert!(!Arc::ptr_eq(&second.0, &refreshed.0));
    assert_eq!(refreshed.0.status_text.as_str(), "changed");
}

#[test]
fn projection_cache_invalidate_forces_refresh() {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let first = cache.resolve_or_project(&mut controller);
    cache.invalidate();
    let second = cache.resolve_or_project(&mut controller);
    assert!(!Arc::ptr_eq(&first.0, &second.0));
    assert_eq!(
        second.1,
        NativeDirtySegments::from_bits(
            NativeDirtySegments::STATUS_BAR
                | NativeDirtySegments::BROWSER_FRAME
                | NativeDirtySegments::BROWSER_ROWS_WINDOW
                | NativeDirtySegments::MAP_PANEL
                | NativeDirtySegments::WAVEFORM_OVERLAY
                | NativeDirtySegments::GLOBAL_STATIC
        )
    );
}

/// Immediate waveform preview parser should accept canonical truthy variants.
#[test]
fn env_truthy_parser_is_case_insensitive_for_immediate_preview_flag() {
    assert!(crate::env_flags::is_truthy("TRUE"));
    assert!(crate::env_flags::is_truthy("on"));
    assert!(crate::env_flags::is_truthy("Yes"));
    assert!(crate::env_flags::is_truthy("  true  "));
    assert!(!crate::env_flags::is_truthy("0"));
    assert!(!crate::env_flags::is_truthy("no"));
    assert!(!crate::env_flags::is_truthy(""));
}

#[cfg(feature = "native-bridge-metrics")]
#[test]
/// Shared env truthy parsing should accept canonical bridge-profile variants.
fn env_truthy_parser_is_case_insensitive_for_bridge_flags() {
    assert!(crate::env_flags::is_truthy("TRUE"));
    assert!(crate::env_flags::is_truthy("on"));
    assert!(crate::env_flags::is_truthy("Yes"));
    assert!(crate::env_flags::is_truthy("  true  "));
    assert!(!crate::env_flags::is_truthy("0"));
    assert!(!crate::env_flags::is_truthy("no"));
    assert!(!crate::env_flags::is_truthy(""));
}
