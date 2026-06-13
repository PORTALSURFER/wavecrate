use super::{selected_column_index, status_bar_right_text, waveform_projection};
use crate::app_core::actions::{NativeMotionModel as MotionModel, NativeNormalizedRangeModel};
use crate::app_core::controller::AppController;
use crate::app_core::state::WaveformSliceBatchProfile;
use crate::app_core::state::{SampleBrowserTab, browser_playback_age_filter_chips};

/// Project motion-only model fields used by animation-phase redraws.
///
/// This path intentionally avoids rebuilding static panel payloads and should
/// stay aligned with corresponding waveform/map/status fields in `project_app_model`.
pub(crate) fn project_motion_model(controller: &mut AppController) -> MotionModel {
    let selected_column = selected_column_index(&controller.ui);
    let fade_overlay =
        waveform_projection::project_waveform_edit_fade_overlay_model(&controller.ui);
    let projected_playhead = waveform_projection::projected_playhead_ratio(controller);
    MotionModel {
        transport_running: controller.is_playing(),
        map_active: matches!(
            SampleBrowserTab::from(controller.ui.browser.active_tab),
            SampleBrowserTab::Map
        ),
        active_rating_filters: active_rating_filter_flags(controller),
        active_playback_age_filters: active_playback_age_filter_flags(controller),
        marked_filter_active: controller.ui.browser.search.marked_only,
        waveform_selection_milli: controller.ui.waveform.selection.map(|selection| {
            NativeNormalizedRangeModel::from_micros(
                waveform_projection::normalized_to_micros(selection.start()),
                waveform_projection::normalized_to_micros(selection.end()),
            )
        }),
        waveform_slices: waveform_projection::project_waveform_slice_previews(&controller.ui)
            .into_iter()
            .collect(),
        waveform_selection_export_flash_nonce: controller.ui.waveform.selection_export_flash_nonce,
        waveform_selection_export_failure_flash_nonce: controller
            .ui
            .waveform
            .selection_export_failure_flash_nonce,
        waveform_edit_selection_apply_flash_nonce: controller
            .ui
            .waveform
            .edit_selection_apply_flash_nonce,
        waveform_edit_selection_milli: waveform_projection::project_waveform_edit_selection_milli(
            &controller.ui,
        ),
        waveform_edit_fade_in_end_milli: fade_overlay.fade_in_end_milli,
        waveform_edit_fade_in_end_micros: fade_overlay.fade_in_end_micros,
        waveform_edit_fade_in_mute_start_milli: fade_overlay.fade_in_mute_start_milli,
        waveform_edit_fade_in_mute_start_micros: fade_overlay.fade_in_mute_start_micros,
        waveform_edit_fade_in_curve_milli: fade_overlay.fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: fade_overlay.fade_out_start_milli,
        waveform_edit_fade_out_start_micros: fade_overlay.fade_out_start_micros,
        waveform_edit_fade_out_mute_end_milli: fade_overlay.fade_out_mute_end_milli,
        waveform_edit_fade_out_mute_end_micros: fade_overlay.fade_out_mute_end_micros,
        waveform_edit_fade_out_curve_milli: fade_overlay.fade_out_curve_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_loop_lock_enabled: controller.ui.waveform.loop_lock_enabled,
        waveform_cursor_milli: controller
            .ui
            .waveform
            .cursor
            .map(waveform_projection::normalized_to_milli),
        waveform_playhead_milli: projected_playhead.map(waveform_projection::normalized_to_milli),
        waveform_playhead_micros: projected_playhead.map(waveform_projection::normalized_to_micros),
        waveform_view_start_milli: waveform_projection::normalized64_to_milli(
            controller.ui.waveform.view.start,
        ),
        waveform_view_end_milli: waveform_projection::normalized64_to_milli(
            controller.ui.waveform.view.end,
        ),
        waveform_view_start_micros: waveform_projection::normalized64_to_micros(
            controller.ui.waveform.view.start,
        ),
        waveform_view_end_micros: waveform_projection::normalized64_to_micros(
            controller.ui.waveform.view.end,
        ),
        waveform_view_start_nanos: waveform_projection::normalized64_to_nanos(
            controller.ui.waveform.view.start,
        ),
        waveform_view_end_nanos: waveform_projection::normalized64_to_nanos(
            controller.ui.waveform.view.end,
        ),
        waveform_tempo_label: controller
            .ui
            .waveform
            .bpm_value
            .map(|bpm| format!("{bpm:.1} BPM")),
        waveform_zoom_label: waveform_zoom_label(controller),
        waveform_image_signature: waveform_projection::effective_waveform_image_signature(
            controller,
        ),
        waveform_loaded_label: waveform_projection::project_waveform_target_label(&controller.ui),
        waveform_loading: controller.ui.waveform.loading.is_some(),
        waveform_transport_hint: waveform_projection::waveform_transport_hint(&controller.ui),
        waveform_compare_anchor_available: controller.ui.compare_anchor.is_some(),
        waveform_compare_anchor_label: controller.ui.waveform.compare_anchor_label.clone(),
        waveform_channel_view: waveform_projection::project_waveform_channel_view_model(
            controller.ui.waveform.channel_view,
        ),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_relative_bpm_grid_enabled: controller.ui.waveform.relative_bpm_grid_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        waveform_exact_duplicate_cleanup_available: controller.ui.waveform.slice_batch_profile
            == WaveformSliceBatchProfile::ExactDuplicateBeats
            && !controller.ui.waveform.slices.is_empty(),
        status_right: status_bar_right_text(selected_column),
    }
}

fn active_rating_filter_flags(controller: &AppController) -> [bool; 8] {
    let mut flags = [false; 8];
    for (index, level) in [-3, -2, -1, 0, 1, 2, 3, 4].into_iter().enumerate() {
        flags[index] = controller.ui.browser.search.rating_filter.contains(&level);
    }
    flags
}

fn active_playback_age_filter_flags(controller: &AppController) -> [bool; 3] {
    let mut flags = [false; 3];
    for (index, chip) in browser_playback_age_filter_chips().into_iter().enumerate() {
        flags[index] = controller
            .ui
            .browser
            .search
            .playback_age_filter
            .contains(&chip);
    }
    flags
}

fn waveform_zoom_label(controller: &AppController) -> Option<String> {
    Some(format!(
        "{:.0}%",
        (100.0
            / (controller.ui.waveform.view.end - controller.ui.waveform.view.start)
                .clamp(0.000_1, 1.0) as f32)
            .round()
            .clamp(100.0, 9999.0)
    ))
}
