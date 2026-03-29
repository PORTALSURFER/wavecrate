use super::super::super::projection_key_encoding::{
    normalized_f32_to_micros, normalized_f32_to_milli, normalized_f64_to_micros,
    normalized_f64_to_milli,
};
use super::super::WaveformProjectionCacheKey;
use crate::app_core::controller::AppController;

/// Build a waveform projection key from the current controller snapshot.
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> WaveformProjectionCacheKey {
    let scalars = derive_waveform_projection_scalars(controller);
    WaveformProjectionCacheKey {
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_selection_start_milli: scalars.selection_start_milli,
        waveform_selection_end_milli: scalars.selection_end_milli,
        waveform_selection_start_micros: scalars.selection_start_micros,
        waveform_selection_end_micros: scalars.selection_end_micros,
        waveform_edit_selection_start_milli: scalars.edit_selection_start_milli,
        waveform_edit_selection_end_milli: scalars.edit_selection_end_milli,
        waveform_edit_selection_start_micros: scalars.edit_selection_start_micros,
        waveform_edit_selection_end_micros: scalars.edit_selection_end_micros,
        waveform_edit_fade_in_end_milli: scalars.edit_fade_in_end_milli,
        waveform_edit_fade_in_mute_start_milli: scalars.edit_fade_in_mute_start_milli,
        waveform_edit_fade_in_curve_milli: scalars.edit_fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: scalars.edit_fade_out_start_milli,
        waveform_edit_fade_out_mute_end_milli: scalars.edit_fade_out_mute_end_milli,
        waveform_edit_fade_out_curve_milli: scalars.edit_fade_out_curve_milli,
        waveform_edit_fade_in_end_micros: scalars.edit_fade_in_end_micros,
        waveform_edit_fade_in_mute_start_micros: scalars.edit_fade_in_mute_start_micros,
        waveform_edit_fade_out_start_micros: scalars.edit_fade_out_start_micros,
        waveform_edit_fade_out_mute_end_micros: scalars.edit_fade_out_mute_end_micros,
        waveform_view_start_milli: scalars.view_start_milli,
        waveform_view_end_milli: scalars.view_end_milli,
        waveform_view_start_micros: scalars.view_start_micros,
        waveform_view_end_micros: scalars.view_end_micros,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_loop_lock_enabled: controller.ui.waveform.loop_lock_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        waveform_channel_view: encode_waveform_channel_view(controller),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_relative_bpm_grid_enabled: controller.ui.waveform.relative_bpm_grid_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
        transport_running: controller.is_playing(),
    }
}

/// Normalized waveform projection values converted to cache-key scalars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WaveformProjectionScalars {
    selection_start_milli: Option<u16>,
    selection_end_milli: Option<u16>,
    selection_start_micros: Option<u32>,
    selection_end_micros: Option<u32>,
    edit_selection_start_milli: Option<u16>,
    edit_selection_end_milli: Option<u16>,
    edit_selection_start_micros: Option<u32>,
    edit_selection_end_micros: Option<u32>,
    edit_fade_in_end_milli: Option<u16>,
    edit_fade_in_mute_start_milli: Option<u16>,
    edit_fade_in_curve_milli: Option<u16>,
    edit_fade_out_start_milli: Option<u16>,
    edit_fade_out_mute_end_milli: Option<u16>,
    edit_fade_out_curve_milli: Option<u16>,
    edit_fade_in_end_micros: Option<u32>,
    edit_fade_in_mute_start_micros: Option<u32>,
    edit_fade_out_start_micros: Option<u32>,
    edit_fade_out_mute_end_micros: Option<u32>,
    view_start_milli: u16,
    view_end_milli: u16,
    view_start_micros: u32,
    view_end_micros: u32,
}

/// Derive normalized waveform projection key fields once for cache-key builders.
fn derive_waveform_projection_scalars(controller: &AppController) -> WaveformProjectionScalars {
    let (selection_start_milli, selection_end_milli) = controller
        .ui
        .waveform
        .selection
        .map(|selection| {
            let start = normalized_f32_to_milli(selection.start());
            let end = normalized_f32_to_milli(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (selection_start_micros, selection_end_micros) = controller
        .ui
        .waveform
        .selection
        .map(|selection| {
            let start = normalized_f32_to_micros(selection.start());
            let end = normalized_f32_to_micros(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (edit_selection_start_milli, edit_selection_end_milli) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = normalized_f32_to_milli(selection.start());
            let end = normalized_f32_to_milli(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (edit_selection_start_micros, edit_selection_end_micros) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = normalized_f32_to_micros(selection.start());
            let end = normalized_f32_to_micros(selection.end());
            (Some(start.min(end)), Some(start.max(end)))
        })
        .unwrap_or((None, None));
    let (edit_fade_in_curve_milli, edit_fade_out_curve_milli) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let fade_in = selection
                .fade_in()
                .map(|fade| normalized_f64_to_milli(f64::from(fade.curve)));
            let fade_out = selection
                .fade_out()
                .map(|fade| normalized_f64_to_milli(f64::from(fade.curve)));
            (fade_in, fade_out)
        })
        .unwrap_or((None, None));
    let (
        edit_fade_in_end_milli,
        edit_fade_in_mute_start_milli,
        edit_fade_out_start_milli,
        edit_fade_out_mute_end_milli,
        edit_fade_in_end_micros,
        edit_fade_in_mute_start_micros,
        edit_fade_out_start_micros,
        edit_fade_out_mute_end_micros,
    ) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return (None, None, None, None, None, None, None, None);
            }
            let fade_in_end = selection.fade_in().map(|fade| {
                normalized_f32_to_milli((start + (width * fade.length)).clamp(start, end))
            });
            let fade_in_mute_start = selection.fade_in().map(|fade| {
                normalized_f32_to_milli((start - (width * fade.mute)).clamp(0.0, start))
            });
            let fade_out_start = selection.fade_out().map(|fade| {
                normalized_f32_to_milli((end - (width * fade.length)).clamp(start, end))
            });
            let fade_out_mute_end = selection
                .fade_out()
                .map(|fade| normalized_f32_to_milli((end + (width * fade.mute)).clamp(end, 1.0)));
            let fade_in_end_micros = selection.fade_in().map(|fade| {
                normalized_f32_to_micros((start + (width * fade.length)).clamp(start, end))
            });
            let fade_in_mute_start_micros = selection.fade_in().map(|fade| {
                normalized_f32_to_micros((start - (width * fade.mute)).clamp(0.0, start))
            });
            let fade_out_start_micros = selection.fade_out().map(|fade| {
                normalized_f32_to_micros((end - (width * fade.length)).clamp(start, end))
            });
            let fade_out_mute_end_micros = selection
                .fade_out()
                .map(|fade| normalized_f32_to_micros((end + (width * fade.mute)).clamp(end, 1.0)));
            (
                fade_in_end,
                fade_in_mute_start,
                fade_out_start,
                fade_out_mute_end,
                fade_in_end_micros,
                fade_in_mute_start_micros,
                fade_out_start_micros,
                fade_out_mute_end_micros,
            )
        })
        .unwrap_or((None, None, None, None, None, None, None, None));
    WaveformProjectionScalars {
        selection_start_milli,
        selection_end_milli,
        selection_start_micros,
        selection_end_micros,
        edit_selection_start_milli,
        edit_selection_end_milli,
        edit_selection_start_micros,
        edit_selection_end_micros,
        edit_fade_in_end_milli,
        edit_fade_in_mute_start_milli,
        edit_fade_in_curve_milli,
        edit_fade_out_start_milli,
        edit_fade_out_mute_end_milli,
        edit_fade_out_curve_milli,
        edit_fade_in_end_micros,
        edit_fade_in_mute_start_micros,
        edit_fade_out_start_micros,
        edit_fade_out_mute_end_micros,
        view_start_milli: normalized_f64_to_milli(controller.ui.waveform.view.start),
        view_end_milli: normalized_f64_to_milli(controller.ui.waveform.view.end),
        view_start_micros: normalized_f64_to_micros(controller.ui.waveform.view.start),
        view_end_micros: normalized_f64_to_micros(controller.ui.waveform.view.end),
    }
}

/// Encode waveform channel-view mode for compact projection keys.
fn encode_waveform_channel_view(controller: &AppController) -> u8 {
    match controller.ui.waveform.channel_view {
        crate::waveform::WaveformChannelView::Mono => 0,
        crate::waveform::WaveformChannelView::SplitStereo => 1,
    }
}
