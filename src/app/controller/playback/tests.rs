use super::*;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::controller::test_support;
use crate::waveform::DecodedWaveform;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn selection_duration_label_uses_loaded_audio() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("clip.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    let label = controller.selection_duration_label(SelectionRange::new(0.25, 0.75));
    assert_eq!(label.as_deref(), Some("2.00 s"));
}

#[test]
fn selection_duration_label_is_absent_without_audio() {
    let (controller, _) = test_support::dummy_controller();
    let label = controller.selection_duration_label(SelectionRange::new(0.0, 1.0));
    assert!(label.is_none());
}

#[test]
fn playhead_progress_updates_position_without_play_state() {
    let (mut controller, _source) = test_support::dummy_controller();

    controller.update_playhead_from_progress(Some(0.42), false);

    assert!(controller.ui.waveform.playhead.visible);
    assert!((controller.ui.waveform.playhead.position - 0.42).abs() < 0.0001);
}

#[test]
fn playhead_progress_completion_hides_playhead() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.waveform.playhead.active_span_end = Some(1.0);

    controller.update_playhead_from_progress(Some(0.9995), false);

    assert!(!controller.ui.waveform.playhead.visible);
    assert!(controller.ui.waveform.playhead.active_span_end.is_none());
}

#[test]
fn normalized_from_milli_clamps_bounds() {
    assert_eq!(waveform_actions::normalized_from_milli(0), 0.0);
    assert_eq!(waveform_actions::normalized_from_milli(455), 0.455);
    assert_eq!(waveform_actions::normalized_from_milli(2000), 1.0);
}

#[test]
fn selection_range_from_milli_clamps_and_orders_bounds() {
    let range = waveform_actions::selection_range_from_milli(750, 250);
    assert_eq!(range.start(), 0.25);
    assert_eq!(range.end(), 0.75);

    let range = waveform_actions::selection_range_from_milli(2000, 0);
    assert_eq!(range.start(), 0.0);
    assert_eq!(range.end(), 1.0);
}

#[test]
fn zoom_steps_from_ui_clamps_to_at_least_one() {
    assert_eq!(waveform_actions::zoom_steps_from_ui(0), 1);
    assert_eq!(waveform_actions::zoom_steps_from_ui(1), 1);
    assert_eq!(waveform_actions::zoom_steps_from_ui(12), 12);
}

/// Seed minimal decoded waveform state so zoom tests can exercise view math.
fn seed_waveform_for_zoom(controller: &mut AppController) {
    controller.sample_view.waveform.size = [240, 24];
    controller.sample_view.waveform.decoded = Some(std::sync::Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: std::sync::Arc::from(vec![0.0; 10_000]),
        analysis_samples: std::sync::Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
}

/// UI zoom should preserve the cursor's relative viewport position as the zoom anchor.
#[test]
fn zoom_steps_from_ui_preserves_cursor_anchor_ratio() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.8,
    };
    controller.ui.waveform.cursor = Some(0.35);

    let before = controller.ui.waveform.view;
    let cursor = f64::from(controller.ui.waveform.cursor.unwrap_or(0.0));
    let before_ratio = (cursor - before.start) / (before.end - before.start);

    controller.zoom_waveform_steps_from_ui(true, 1);

    let after = controller.ui.waveform.view;
    let after_ratio = (cursor - after.start) / (after.end - after.start);
    assert!((before_ratio - after_ratio).abs() < 1.0e-4);
}

/// Pointer-anchored UI zoom should preserve the hovered ratio across zoom steps.
#[test]
fn zoom_steps_from_ui_with_anchor_ratio_preserves_pointer_position() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.8,
    };
    controller.ui.waveform.cursor = Some(0.9);
    let anchor_ratio_micros = 250_000;
    let anchor = 0.35f64;

    controller.zoom_waveform_steps_from_ui_with_anchor(true, 1, Some(anchor_ratio_micros));

    let after = controller.ui.waveform.view;
    let after_ratio = (anchor - after.start) / (after.end - after.start);
    assert!((after_ratio - 0.25).abs() < 1.0e-6);
    assert!(
        controller
            .ui
            .waveform
            .cursor
            .is_some_and(|cursor| (f64::from(cursor) - anchor).abs() < 1.0e-6)
    );
}

/// UI zoom should initialize cursor at view center when none exists.
#[test]
fn zoom_steps_from_ui_initializes_cursor_at_view_center() {
    let (mut controller, _source) = test_support::dummy_controller();
    seed_waveform_for_zoom(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.1,
        end: 0.9,
    };
    controller.ui.waveform.cursor = None;

    controller.zoom_waveform_steps_from_ui(true, 1);

    assert_eq!(controller.ui.waveform.cursor, Some(0.5));
}

/// Tiny floating-point drift should not be treated as a waveform view change.
#[test]
fn waveform_view_changed_ignores_tiny_float_noise() {
    let base = crate::app::state::WaveformView {
        start: 0.25,
        end: 0.75,
    };
    let nearly_equal = crate::app::state::WaveformView {
        start: 0.25 + (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
        end: 0.75 - (WAVEFORM_VIEW_NOOP_EPSILON * 0.25),
    };
    assert!(!waveform_actions::waveform_view_changed(base, nearly_equal));
}

/// Cursor updates should no-op when the cursor is unchanged and waveform is focused.
#[test]
fn set_waveform_cursor_milli_noops_when_unchanged_and_focused() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
    controller.ui.waveform.cursor = Some(0.5);
    let previous_nav = std::time::Instant::now() - std::time::Duration::from_millis(2);
    controller.ui.waveform.cursor_last_navigation_at = Some(previous_nav);

    controller.set_waveform_cursor_milli(500);

    assert_eq!(controller.ui.waveform.cursor, Some(0.5));
    assert_eq!(
        controller.ui.waveform.cursor_last_navigation_at,
        Some(previous_nav)
    );
}

/// Selection updates should no-op when the range is unchanged and waveform is focused.
#[test]
fn set_waveform_selection_range_milli_noops_when_unchanged_and_focused() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.ui.focus.context = crate::app::state::FocusContext::Waveform;
    let range = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(250, 750);

    assert_eq!(controller.selection_state.range.range(), Some(range));
    assert_eq!(controller.ui.waveform.selection, Some(range));
}

#[test]
/// Selection-edge resize from native milli actions should BPM-snap the moving endpoint.
fn set_waveform_selection_range_milli_snaps_resize_endpoint_when_bpm_snap_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.8);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(800, 333);

    let updated = controller.ui.waveform.selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.375).abs() < 0.001);
    assert!((updated.end() - 0.8).abs() < 0.001);
}

#[test]
/// In-bounds selection updates should still BPM-snap even when they land on the visible edge.
fn set_waveform_selection_range_milli_snaps_visible_edge_without_preserve_flag() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_view_edge.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(60.0);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.18,
        end: 0.68,
    };
    let range = SelectionRange::new(0.3, 0.5);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(500, 180);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((updated.start() - 0.25).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
}

#[test]
/// Native out-of-bounds drags should pin to the visible left waveform edge.
fn set_waveform_selection_range_milli_preserves_left_view_edge_when_requested() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap_view_edge_preserved.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(60.0);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.18,
        end: 0.68,
    };
    let range = SelectionRange::new(0.3, 0.5);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli_with_edge_policy(500, 180, true);

    let updated = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((updated.start() - 0.18).abs() < 0.001);
    assert!((updated.end() - 0.5).abs() < 0.001);
}

#[test]
/// Selection-translation updates should snap the moved range to BPM steps.
fn set_waveform_selection_range_milli_snaps_translated_range_when_bpm_snap_enabled() {
    let (mut controller, source) = test_support::dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("snap.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 4.0,
        sample_rate: 48_000,
    });
    controller.ui.waveform.bpm_snap_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);
    let range = SelectionRange::new(0.2, 0.4);
    controller.selection_state.range.set_range(Some(range));
    controller.ui.waveform.selection = Some(range);

    controller.set_waveform_selection_range_milli(260, 460);

    let updated = controller.ui.waveform.selection;
    assert!(updated.is_some());
    let updated = updated.unwrap_or(range);
    assert!((updated.start() - 0.25).abs() < 0.001);
    assert!((updated.end() - 0.45).abs() < 0.001);
}

/// Clearing edit selection via native helper should clear edit state and preserve focus.
#[test]
fn clear_waveform_edit_selection_with_focus_clears_edit_selection() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller
        .selection_state
        .edit_range
        .set_range(Some(SelectionRange::new(0.1, 0.4)));
    controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.1, 0.4));

    controller.clear_waveform_edit_selection_with_focus();

    assert!(controller.selection_state.edit_range.range().is_none());
    assert!(controller.ui.waveform.edit_selection.is_none());
}

/// Deferred playback-age writes should remain queued until debounce expires.
#[test]
fn deferred_pending_age_update_commit_waits_for_deadline() {
    let (mut controller, source) = test_support::prepare_with_source_and_wav_entries(vec![
        test_support::sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        test_support::sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.audio.pending_age_update = Some(PendingAgeUpdate {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        played_at: 123,
    });

    controller.defer_pending_age_update_commit_if_path_changes(Path::new("two.wav"));
    assert!(controller.runtime.pending_age_update_commit.is_some());

    controller.flush_pending_age_update_commit();
    assert!(controller.runtime.pending_age_update_commit.is_some());
}

/// Queued waveform seek updates should defer commit-side playback work.
#[test]
fn queue_waveform_seek_milli_defers_commit_until_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();

    controller.queue_waveform_seek_milli(500);

    assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(500));
    controller.flush_pending_waveform_seek_commit();
    assert_eq!(controller.pending_waveform_seek_milli_for_test(), Some(500));
}

/// Expired deferred waveform seek commits should clear queued seek state.
#[test]
fn flush_pending_waveform_seek_commit_clears_queue_after_deadline() {
    let (mut controller, _source) = test_support::dummy_controller();
    controller.queue_waveform_seek_milli(750);
    controller.runtime.pending_waveform_seek_not_before =
        Some(Instant::now() - Duration::from_millis(1));

    controller.flush_pending_waveform_seek_commit();

    assert!(controller.runtime.pending_waveform_seek_milli.is_none());
    assert!(
        controller
            .runtime
            .pending_waveform_seek_not_before
            .is_none()
    );
}
