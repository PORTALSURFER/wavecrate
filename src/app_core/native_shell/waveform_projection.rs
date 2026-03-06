//! Waveform panel and waveform chrome projection helpers.

use super::*;

/// Projected edit-fade overlay endpoints and curve values in normalized milli-space.
pub(super) struct WaveformEditFadeOverlayMilli {
    /// End position of the fade-in ramp within the edit selection.
    pub(super) fade_in_end_milli: Option<u16>,
    /// Start position of the fade-in mute segment before the edit selection.
    pub(super) fade_in_mute_start_milli: Option<u16>,
    /// Fade-in curve amount mapped into milli-space.
    pub(super) fade_in_curve_milli: Option<u16>,
    /// Start position of the fade-out ramp within the edit selection.
    pub(super) fade_out_start_milli: Option<u16>,
    /// End position of the fade-out mute segment after the edit selection.
    pub(super) fade_out_mute_end_milli: Option<u16>,
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
    let fade_overlay = project_waveform_edit_fade_overlay_milli(ui);
    WaveformPanelModel {
        loaded_label: ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_to_milli(ui.waveform.playhead.position)),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        edit_selection_milli: project_waveform_edit_selection_milli(ui),
        edit_fade_in_end_milli: fade_overlay.fade_in_end_milli,
        edit_fade_in_mute_start_milli: fade_overlay.fade_in_mute_start_milli,
        edit_fade_in_curve_milli: fade_overlay.fade_in_curve_milli,
        edit_fade_out_start_milli: fade_overlay.fade_out_start_milli,
        edit_fade_out_mute_end_milli: fade_overlay.fade_out_mute_end_milli,
        edit_fade_out_curve_milli: fade_overlay.fade_out_curve_milli,
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        loop_enabled: ui.waveform.loop_enabled,
        tempo_label: ui.waveform.bpm_value.map(|bpm| format!("{bpm:.1} BPM")),
        zoom_label: Some(format!("{zoom_percent:.0}%")),
        waveform_image_signature: ui.waveform.waveform_image_signature,
        waveform_image: project_waveform_image(controller),
    }
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
            .and_then(waveform_image_to_native_rgba)
    });
    controller.projected_waveform_image_signature = signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

/// Convert a rendered waveform image into the native immutable RGBA payload.
fn waveform_image_to_native_rgba(image: &crate::waveform::WaveformImage) -> Option<Arc<ImageRgba>> {
    if image.size[0] == 0 || image.size[1] == 0 {
        return None;
    }
    let mut pixels = Vec::with_capacity(
        image.size[0]
            .saturating_mul(image.size[1])
            .saturating_mul(4),
    );
    for pixel in &image.pixels {
        pixels.push(pixel.r());
        pixels.push(pixel.g());
        pixels.push(pixel.b());
        pixels.push(pixel.a());
    }
    ImageRgba::new(image.size[0], image.size[1], pixels).map(Arc::new)
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
        NormalizedRangeModel::new(
            normalized_to_milli(selection.start()),
            normalized_to_milli(selection.end()),
        )
    })
}

/// Project edit fade-handle positions into normalized milli-space.
pub(super) fn project_waveform_edit_fade_overlay_milli(
    ui: &UiState,
) -> WaveformEditFadeOverlayMilli {
    ui.waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return WaveformEditFadeOverlayMilli {
                    fade_in_end_milli: None,
                    fade_in_mute_start_milli: None,
                    fade_in_curve_milli: None,
                    fade_out_start_milli: None,
                    fade_out_mute_end_milli: None,
                    fade_out_curve_milli: None,
                };
            }
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
            WaveformEditFadeOverlayMilli {
                fade_in_end_milli,
                fade_in_mute_start_milli,
                fade_in_curve_milli,
                fade_out_start_milli,
                fade_out_mute_end_milli,
                fade_out_curve_milli,
            }
        })
        .unwrap_or(WaveformEditFadeOverlayMilli {
            fade_in_end_milli: None,
            fade_in_mute_start_milli: None,
            fade_in_curve_milli: None,
            fade_out_start_milli: None,
            fade_out_mute_end_milli: None,
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
