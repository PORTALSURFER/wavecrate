//! Waveform panel and waveform chrome projection helpers.

use super::*;

/// Projected edit-fade overlay endpoints and curve values for the native waveform shell.
pub(super) struct WaveformEditFadeOverlayModel {
    /// End position of the fade-in ramp within the edit selection.
    pub(super) fade_in_end_milli: Option<u16>,
    /// End position of the fade-in ramp within the edit selection at micro precision.
    pub(super) fade_in_end_micros: Option<u32>,
    /// Start position of the fade-in mute segment before the edit selection.
    pub(super) fade_in_mute_start_milli: Option<u16>,
    /// Start position of the fade-in mute segment before the edit selection at micro precision.
    pub(super) fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve amount mapped into milli-space.
    pub(super) fade_in_curve_milli: Option<u16>,
    /// Start position of the fade-out ramp within the edit selection.
    pub(super) fade_out_start_milli: Option<u16>,
    /// Start position of the fade-out ramp within the edit selection at micro precision.
    pub(super) fade_out_start_micros: Option<u32>,
    /// End position of the fade-out mute segment after the edit selection.
    pub(super) fade_out_mute_end_milli: Option<u16>,
    /// End position of the fade-out mute segment after the edit selection at micro precision.
    pub(super) fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve amount mapped into milli-space.
    pub(super) fade_out_curve_milli: Option<u16>,
}

/// Project waveform panel state, selection handles, and cached raster payloads.
///
/// This projection reads controller/UI waveform state and preserves raster reuse
/// by honoring waveform image signatures when available.
pub(crate) fn project_waveform_model(controller: &mut AppController) -> WaveformPanelModel {
    let ui = &controller.ui;
    let view_span = (ui.waveform.view.end - ui.waveform.view.start).clamp(0.000_1, 1.0) as f32;
    let zoom_percent = (100.0 / view_span).round().clamp(100.0, 9999.0);
    let fade_overlay = project_waveform_edit_fade_overlay_model(ui);
    let projected_playhead = projected_playhead_ratio(controller);
    WaveformPanelModel {
        loaded_label: ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        loading: ui.waveform.loading.is_some(),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: projected_playhead.map(normalized_to_milli),
        playhead_micros: projected_playhead.map(normalized_to_micros),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::from_micros(
                normalized_to_micros(selection.start()),
                normalized_to_micros(selection.end()),
            )
        }),
        slices: project_waveform_slice_previews(ui),
        selection_export_flash_nonce: ui.waveform.selection_export_flash_nonce,
        selection_export_failure_flash_nonce: ui.waveform.selection_export_failure_flash_nonce,
        edit_selection_apply_flash_nonce: ui.waveform.edit_selection_apply_flash_nonce,
        edit_selection_milli: project_waveform_edit_selection_milli(ui),
        edit_fade_in_end_milli: fade_overlay.fade_in_end_milli,
        edit_fade_in_end_micros: fade_overlay.fade_in_end_micros,
        edit_fade_in_mute_start_milli: fade_overlay.fade_in_mute_start_milli,
        edit_fade_in_mute_start_micros: fade_overlay.fade_in_mute_start_micros,
        edit_fade_in_curve_milli: fade_overlay.fade_in_curve_milli,
        edit_fade_out_start_milli: fade_overlay.fade_out_start_milli,
        edit_fade_out_start_micros: fade_overlay.fade_out_start_micros,
        edit_fade_out_mute_end_milli: fade_overlay.fade_out_mute_end_milli,
        edit_fade_out_mute_end_micros: fade_overlay.fade_out_mute_end_micros,
        edit_fade_out_curve_milli: fade_overlay.fade_out_curve_milli,
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        view_start_micros: normalized64_to_micros(ui.waveform.view.start),
        view_end_micros: normalized64_to_micros(ui.waveform.view.end),
        view_start_nanos: normalized64_to_nanos(ui.waveform.view.start),
        view_end_nanos: normalized64_to_nanos(ui.waveform.view.end),
        beat_step_micros: project_waveform_beat_step_micros(controller),
        bpm_grid_origin_micros: project_waveform_bpm_grid_origin_micros(ui),
        loop_enabled: ui.waveform.loop_enabled,
        tempo_label: ui.waveform.bpm_value.map(|bpm| format!("{bpm:.1} BPM")),
        zoom_label: Some(format!("{zoom_percent:.0}%")),
        waveform_image_signature: ui.waveform.waveform_image_signature,
        waveform_image: project_waveform_image(controller),
    }
}

/// Resolve the waveform playhead ratio used by native projection.
///
/// When transport is actively playing, prefer the live audio-player progress so
/// motion-only redraws are not limited by the UI state's playhead update cadence.
/// Fall back to the last UI playhead snapshot for paused or unavailable players.
pub(super) fn projected_playhead_ratio(controller: &AppController) -> Option<f32> {
    resolve_projected_playhead_ratio(
        controller.ui.waveform.playhead.visible,
        controller.ui.waveform.playhead.position,
        controller.live_playback_progress(),
    )
}

/// Resolve the preferred playhead ratio from UI and live transport inputs.
///
/// The native runtime uses this to keep playhead motion smooth while preserving
/// the last known UI snapshot whenever transport is idle or live progress is
/// temporarily unavailable.
pub(super) fn resolve_projected_playhead_ratio(
    playhead_visible: bool,
    ui_ratio: f32,
    live_progress: Option<f32>,
) -> Option<f32> {
    if !playhead_visible {
        return None;
    }
    live_progress
        .filter(|progress| progress.is_finite())
        .map(|progress| progress.clamp(0.0, 1.0))
        .or(Some(ui_ratio.clamp(0.0, 1.0)))
}

/// Project normalized quarter-note beat spacing for BPM-aligned waveform overlays.
fn project_waveform_beat_step_micros(controller: &AppController) -> Option<u32> {
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let normalized_step = 60.0 / bpm / duration;
    (normalized_step.is_finite() && normalized_step > 0.0)
        .then_some(normalized_to_micros(normalized_step))
}

/// Project the active or persisted BPM grid origin into normalized micro space.
fn project_waveform_bpm_grid_origin_micros(ui: &UiState) -> u32 {
    let origin = ui
        .waveform
        .selection
        .map(|selection| selection.start())
        .unwrap_or(ui.waveform.last_bpm_grid_origin)
        .clamp(0.0, 1.0);
    normalized_to_micros(origin)
}

/// Reuse or rebuild the projected waveform raster payload for the native model.
fn project_waveform_image(controller: &mut AppController) -> Option<Arc<ImageRgba>> {
    let signature = controller.ui.waveform.waveform_image_signature;
    let has_source_image = controller.ui.waveform.image.is_some();
    let has_cached_image = controller.projected_waveform_image.is_some();
    if signature.is_some()
        && controller.projected_waveform_image_signature == signature
        && has_source_image == has_cached_image
    {
        return controller.projected_waveform_image.clone();
    }
    // Producer-side waveform rendering now publishes shared immutable RGBA payloads and
    // versioned identities. Keep a projection-side fallback for tests/manual image assignment.
    let projected_waveform_image = controller.projected_waveform_image.clone().or_else(|| {
        controller
            .ui
            .waveform
            .image
            .as_ref()
            .and_then(super::waveform_image_to_native_rgba)
    });
    controller.projected_waveform_image_signature = signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

/// Project waveform chrome labels and action-hint copy.
pub(crate) fn project_waveform_chrome_model(ui: &UiState) -> WaveformChromeModel {
    WaveformChromeModel {
        transport_hint: if ui.waveform.loop_enabled {
            String::from("Loop enabled")
        } else {
            String::from("Loop disabled")
        },
        channel_view: project_waveform_channel_view_model(ui.waveform.channel_view),
        normalized_audition_enabled: ui.waveform.normalized_audition_enabled,
        bpm_snap_enabled: ui.waveform.bpm_snap_enabled,
        transient_snap_enabled: ui.waveform.transient_snap_enabled,
        transient_markers_enabled: ui.waveform.transient_markers_enabled,
        slice_mode_enabled: ui.waveform.slice_mode_enabled,
    }
}

/// Project edit-selection bounds into normalized milli-space.
pub(super) fn project_waveform_edit_selection_milli(ui: &UiState) -> Option<NormalizedRangeModel> {
    ui.waveform.edit_selection.map(|selection| {
        NormalizedRangeModel::from_micros(
            normalized_to_micros(selection.start()),
            normalized_to_micros(selection.end()),
        )
    })
}

/// Project waveform slice previews into the native runtime model.
pub(super) fn project_waveform_slice_previews(
    ui: &UiState,
) -> Vec<radiant::app::WaveformSlicePreviewModel> {
    ui.waveform
        .slices
        .iter()
        .enumerate()
        .map(|(index, slice)| radiant::app::WaveformSlicePreviewModel {
            range: NormalizedRangeModel::from_micros(
                normalized_to_micros(slice.start()),
                normalized_to_micros(slice.end()),
            ),
            selected: ui.waveform.selected_slices.contains(&index),
            focused: ui.waveform.slice_review.focused_index == Some(index),
            marked_for_export: ui.waveform.slice_review.marked_indices.contains(&index),
        })
        .collect()
}

/// Project edit fade-handle positions into normalized milli and micro space.
pub(super) fn project_waveform_edit_fade_overlay_model(
    ui: &UiState,
) -> WaveformEditFadeOverlayModel {
    ui.waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return WaveformEditFadeOverlayModel {
                    fade_in_end_milli: None,
                    fade_in_end_micros: None,
                    fade_in_mute_start_milli: None,
                    fade_in_mute_start_micros: None,
                    fade_in_curve_milli: None,
                    fade_out_start_milli: None,
                    fade_out_start_micros: None,
                    fade_out_mute_end_milli: None,
                    fade_out_mute_end_micros: None,
                    fade_out_curve_milli: None,
                };
            }
            let fade_in_end = selection
                .fade_in()
                .map(|fade| (start + (width * fade.length)).clamp(start, end));
            let fade_in_mute_start = selection
                .fade_in()
                .map(|fade| (start - (width * fade.mute)).clamp(0.0, start));
            let fade_out_start = selection
                .fade_out()
                .map(|fade| (end - (width * fade.length)).clamp(start, end));
            let fade_out_mute_end = selection
                .fade_out()
                .map(|fade| (end + (width * fade.mute)).clamp(end, 1.0));
            let fade_in_end_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli((start + (width * fade.length)).clamp(start, end)));
            let fade_in_mute_start_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli((start - (width * fade.mute)).clamp(0.0, start)));
            let fade_in_curve_milli = selection
                .fade_in()
                .map(|fade| normalized_to_milli(fade.curve));
            let fade_out_start_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli((end - (width * fade.length)).clamp(start, end)));
            let fade_out_mute_end_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli((end + (width * fade.mute)).clamp(end, 1.0)));
            let fade_out_curve_milli = selection
                .fade_out()
                .map(|fade| normalized_to_milli(fade.curve));
            WaveformEditFadeOverlayModel {
                fade_in_end_milli,
                fade_in_end_micros: fade_in_end.map(normalized_to_micros),
                fade_in_mute_start_milli,
                fade_in_mute_start_micros: fade_in_mute_start.map(normalized_to_micros),
                fade_in_curve_milli,
                fade_out_start_milli,
                fade_out_start_micros: fade_out_start.map(normalized_to_micros),
                fade_out_mute_end_milli,
                fade_out_mute_end_micros: fade_out_mute_end.map(normalized_to_micros),
                fade_out_curve_milli,
            }
        })
        .unwrap_or(WaveformEditFadeOverlayModel {
            fade_in_end_milli: None,
            fade_in_end_micros: None,
            fade_in_mute_start_milli: None,
            fade_in_mute_start_micros: None,
            fade_in_curve_milli: None,
            fade_out_start_milli: None,
            fade_out_start_micros: None,
            fade_out_mute_end_milli: None,
            fade_out_mute_end_micros: None,
            fade_out_curve_milli: None,
        })
}

/// Translate local waveform channel-view settings into native runtime model enums.
pub(super) fn project_waveform_channel_view_model(
    channel_view: crate::waveform::WaveformChannelView,
) -> radiant::app::WaveformChannelViewModel {
    match channel_view {
        crate::waveform::WaveformChannelView::Mono => radiant::app::WaveformChannelViewModel::Mono,
        crate::waveform::WaveformChannelView::SplitStereo => {
            radiant::app::WaveformChannelViewModel::Stereo
        }
    }
}

/// Convert normalized `f32` scalar values to millisecond-style thousandths.
pub(super) fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert normalized `f32` scalar values to micro-style millionths.
pub(super) fn normalized_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert normalized `f64` scalar values to millisecond-style thousandths.
pub(super) fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert normalized `f64` scalar values to micro-style millionths.
pub(super) fn normalized64_to_micros(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert normalized `f64` scalar values to nano-style billionths.
pub(super) fn normalized64_to_nanos(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000_000.0).round() as u32
}
