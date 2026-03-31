use super::*;
use crate::app::state::WaveformSliceBatchProfile;

/// Live transport progress should override stale UI playhead snapshots during motion pulls.
#[test]
fn resolve_projected_playhead_ratio_prefers_live_transport_progress() {
    let resolved =
        waveform_projection::resolve_projected_playhead_ratio(true, 0.125, Some(0.625_5))
            .expect("playhead ratio");
    assert!((resolved - 0.625_5).abs() < 1.0e-6);

    let fallback =
        waveform_projection::resolve_projected_playhead_ratio(true, 0.125, Some(f32::NAN))
            .expect("fallback ratio");
    assert!((fallback - 0.125).abs() < 1.0e-6);

    assert!(
        waveform_projection::resolve_projected_playhead_ratio(false, 0.125, Some(0.625)).is_none()
    );
}

/// Waveform projection should derive tempo and zoom labels from UI waveform state.
#[test]
fn waveform_projection_exposes_tempo_and_zoom_labels() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.bpm_value = Some(128.0);
    controller.ui.waveform.view.start = 0.25;
    controller.ui.waveform.view.end = 0.75;
    controller.set_loaded_audio_duration_for_tests(4.0);
    let projected = project_waveform_model(&mut controller);
    assert_eq!(projected.tempo_label.as_deref(), Some("128.0 BPM"));
    assert_eq!(projected.zoom_label.as_deref(), Some("200%"));
    assert_eq!(projected.beat_step_micros, Some(117_188));
    assert_eq!(projected.bpm_grid_origin_micros, 0);
    assert!(projected.waveform_image.is_none());
}

/// Waveform projection should pass through raster payload bytes unchanged when present.
#[test]
fn waveform_projection_passes_raster_image_payload() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.image = Some(crate::waveform::WaveformImage {
        size: [2, 1],
        pixels: vec![
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 40),
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(11, 21, 31, 41),
        ],
    });
    let projected = project_waveform_model(&mut controller);
    let waveform_image = projected
        .waveform_image
        .as_ref()
        .expect("waveform image should be projected");
    assert_eq!(waveform_image.width, 2);
    assert_eq!(waveform_image.height, 1);
    assert_eq!(
        waveform_image.pixels.as_ref(),
        &[10, 20, 30, 40, 11, 21, 31, 41]
    );
}

#[test]
fn waveform_projection_marks_loading_state_for_native_shell() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.loading = Some(std::path::PathBuf::from("pending.wav"));

    let projected = project_waveform_model(&mut controller);

    assert!(projected.loading);
}

#[test]
/// Waveform projection should expose edit fade handle positions when fades are configured.
fn waveform_projection_includes_edit_fade_handles() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.edit_selection = Some(
        crate::selection::SelectionRange::new(0.2, 0.8)
            .with_fade_in(0.25, 0.75)
            .with_fade_in_mute(0.1)
            .with_fade_out(0.5, 0.25)
            .with_fade_out_mute(0.2),
    );

    let projected = project_waveform_model(&mut controller);
    assert_eq!(
        projected.edit_selection_milli,
        Some(NormalizedRangeModel::new(200, 800))
    );
    assert_eq!(projected.edit_fade_in_end_milli, Some(350));
    assert_eq!(projected.edit_fade_in_mute_start_milli, Some(140));
    assert_eq!(projected.edit_fade_in_curve_milli, Some(750));
    assert_eq!(projected.edit_fade_out_start_milli, Some(500));
    assert_eq!(projected.edit_fade_out_mute_end_milli, Some(920));
    assert_eq!(projected.edit_fade_out_curve_milli, Some(250));
}

#[test]
fn waveform_projection_preserves_selection_micro_precision() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.relative_bpm_grid_enabled = true;
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.5004, 0.5006));

    let projected = project_waveform_model(&mut controller);
    let selection = projected.selection_milli.expect("projected selection");

    assert_eq!(selection.start_micros, 500_400);
    assert_eq!(selection.end_micros, 500_600);
    assert_eq!(projected.bpm_grid_origin_micros, 500_400);
}

#[test]
fn waveform_projection_falls_back_to_persisted_bpm_grid_origin_without_selection() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.relative_bpm_grid_enabled = true;
    controller.ui.waveform.last_bpm_grid_origin = 0.375;

    let projected = project_waveform_model(&mut controller);

    assert_eq!(projected.bpm_grid_origin_micros, 375_000);
}

#[test]
fn waveform_projection_keeps_global_bpm_grid_origin_at_zero() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
    controller.ui.waveform.relative_bpm_grid_enabled = false;
    controller.ui.waveform.last_bpm_grid_origin = 0.375;
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.5004, 0.5006));

    let projected = project_waveform_model(&mut controller);

    assert_eq!(projected.bpm_grid_origin_micros, 0);
}

/// Waveform chrome projection should mirror loop/channel/toggle state into native labels.
#[test]
fn waveform_chrome_projection_reflects_loop_hint() {
    let mut ui = UiState::default();
    ui.waveform.loop_enabled = false;
    ui.waveform.loop_lock_enabled = false;
    ui.waveform.channel_view = crate::waveform::WaveformChannelView::Mono;
    let projected = project_waveform_chrome_model(&ui);
    assert_eq!(projected.transport_hint, "Loop disabled");
    assert!(!projected.compare_anchor_available);
    assert!(projected.compare_anchor_label.is_none());
    assert!(!projected.loop_lock_enabled);
    assert_eq!(
        projected.channel_view,
        radiant::app::WaveformChannelViewModel::Mono
    );
    assert!(!projected.normalized_audition_enabled);
    assert!(!projected.bpm_snap_enabled);
    assert!(!projected.relative_bpm_grid_enabled);
    assert!(!projected.transient_snap_enabled);
    assert!(projected.transient_markers_enabled);
    assert!(!projected.slice_mode_enabled);

    ui.waveform.loop_lock_enabled = true;
    let projected = project_waveform_chrome_model(&ui);
    assert_eq!(projected.transport_hint, "Loop locked off");
    assert!(projected.loop_lock_enabled);

    ui.waveform.loop_enabled = true;
    ui.compare_anchor = Some(crate::app::state::CompareAnchorState {
        source_id: crate::sample_sources::SourceId::new(),
        relative_path: std::path::PathBuf::from("anchor.wav"),
        label: String::from("anchor.wav"),
    });
    ui.waveform.compare_anchor_label = Some(String::from("anchor.wav"));
    ui.waveform.channel_view = crate::waveform::WaveformChannelView::SplitStereo;
    ui.waveform.normalized_audition_enabled = true;
    ui.waveform.bpm_snap_enabled = true;
    ui.waveform.relative_bpm_grid_enabled = true;
    ui.waveform.transient_snap_enabled = true;
    ui.waveform.transient_markers_enabled = false;
    ui.waveform.slice_mode_enabled = true;
    let projected = project_waveform_chrome_model(&ui);
    assert_eq!(projected.transport_hint, "Loop locked on");
    assert_eq!(
        projected.channel_view,
        radiant::app::WaveformChannelViewModel::Stereo
    );
    assert!(projected.compare_anchor_available);
    assert_eq!(
        projected.compare_anchor_label.as_deref(),
        Some("anchor.wav")
    );
    assert!(projected.loop_lock_enabled);
    assert!(projected.normalized_audition_enabled);
    assert!(projected.bpm_snap_enabled);
    assert!(projected.relative_bpm_grid_enabled);
    assert!(projected.transient_snap_enabled);
    assert!(!projected.transient_markers_enabled);
    assert!(projected.slice_mode_enabled);
    assert!(!projected.exact_duplicate_cleanup_available);
}

#[test]
fn waveform_chrome_projection_marks_exact_duplicate_cleanup_availability() {
    let mut ui = UiState::default();
    ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
    ui.waveform
        .slices
        .push(crate::selection::SelectionRange::new(0.2, 0.4));

    let projected = project_waveform_chrome_model(&ui);

    assert!(projected.exact_duplicate_cleanup_available);
}
