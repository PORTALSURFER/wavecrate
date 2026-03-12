use super::super::projection_key_encoding::encode_browser_filter;
use super::NativeProjectionCacheKey;
use crate::app_core::controller::AppController;

mod browser;
mod map;
mod non_segment;
mod shared;
mod status;
mod waveform;

/// Build the full projection cache key from current controller state.
pub(super) fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    let browser_frame_key = build_browser_frame_projection_key(controller);
    let browser_rows_key = build_browser_rows_projection_key(controller);
    let map_key = build_map_projection_key(controller);
    let waveform_key = build_waveform_projection_key(controller);
    let non_segment_static_key = build_non_segment_static_projection_key(controller);
    let options_panel = crate::app_core::native_shell::project_options_panel_model(&controller.ui);
    NativeProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        sources_selected: non_segment_static_key.sources_selected,
        sources_len: non_segment_static_key.sources_len,
        folder_rows_len: non_segment_static_key.folder_rows_len,
        folder_focused: non_segment_static_key.folder_focused,
        folder_search_revision: non_segment_static_key.folder_search_revision,
        browser_visible_len: browser_frame_key.browser_visible_len,
        browser_visible_rows_revision: browser_rows_key.browser_visible_rows_revision,
        browser_selected_visible: browser_frame_key.browser_selected_visible,
        browser_anchor_visible: browser_frame_key.browser_anchor_visible,
        browser_autoscroll: browser_frame_key.browser_autoscroll,
        browser_view_window_start: browser_frame_key.browser_view_window_start,
        browser_render_window_start: browser_rows_key.browser_render_window_start,
        browser_selected_paths_len: browser_frame_key.browser_selected_paths_len,
        browser_selected_paths_revision: browser_rows_key.browser_selected_paths_revision,
        browser_search_revision: browser_frame_key.browser_search_revision,
        browser_filter: encode_browser_filter(controller.ui.browser.filter),
        browser_sort: browser_frame_key.browser_sort,
        browser_tab: browser_frame_key.browser_tab,
        progress_visible: controller.ui.progress.visible,
        progress_completed: controller.ui.progress.completed,
        progress_total: controller.ui.progress.total,
        prompt_active: controller.ui.browser.pending_action.is_some()
            || controller.ui.sources.folders.pending_action.is_some()
            || controller.ui.sources.folders.new_folder.is_some()
            || controller.ui.waveform.pending_destructive.is_some(),
        drag_active: controller.ui.drag.payload.is_some(),
        options_panel_visible: options_panel.visible,
        options_panel_input_monitoring_enabled: options_panel.input_monitoring_enabled,
        options_panel_advance_after_rating_enabled: options_panel.advance_after_rating_enabled,
        options_panel_destructive_yolo_mode_enabled: options_panel.destructive_yolo_mode_enabled,
        options_panel_invert_waveform_scroll_enabled: options_panel.invert_waveform_scroll_enabled,
        options_panel_trash_folder_hash: controller
            .ui
            .trash_folder
            .as_ref()
            .map(|path| shared::hash_path_for_projection_key(path.as_path())),
        waveform_signature: waveform_key.waveform_signature,
        waveform_selection_start_milli: waveform_key.waveform_selection_start_milli,
        waveform_selection_end_milli: waveform_key.waveform_selection_end_milli,
        waveform_selection_start_micros: waveform_key.waveform_selection_start_micros,
        waveform_selection_end_micros: waveform_key.waveform_selection_end_micros,
        waveform_edit_selection_start_milli: waveform_key.waveform_edit_selection_start_milli,
        waveform_edit_selection_end_milli: waveform_key.waveform_edit_selection_end_milli,
        waveform_edit_selection_start_micros: waveform_key.waveform_edit_selection_start_micros,
        waveform_edit_selection_end_micros: waveform_key.waveform_edit_selection_end_micros,
        waveform_edit_fade_in_end_milli: waveform_key.waveform_edit_fade_in_end_milli,
        waveform_edit_fade_in_mute_start_milli: waveform_key.waveform_edit_fade_in_mute_start_milli,
        waveform_edit_fade_in_curve_milli: waveform_key.waveform_edit_fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: waveform_key.waveform_edit_fade_out_start_milli,
        waveform_edit_fade_out_mute_end_milli: waveform_key.waveform_edit_fade_out_mute_end_milli,
        waveform_edit_fade_out_curve_milli: waveform_key.waveform_edit_fade_out_curve_milli,
        waveform_edit_fade_in_end_micros: waveform_key.waveform_edit_fade_in_end_micros,
        waveform_edit_fade_in_mute_start_micros: waveform_key
            .waveform_edit_fade_in_mute_start_micros,
        waveform_edit_fade_out_start_micros: waveform_key.waveform_edit_fade_out_start_micros,
        waveform_edit_fade_out_mute_end_micros: waveform_key.waveform_edit_fade_out_mute_end_micros,
        waveform_view_start_milli: waveform_key.waveform_view_start_milli,
        waveform_view_end_milli: waveform_key.waveform_view_end_milli,
        waveform_view_start_micros: waveform_key.waveform_view_start_micros,
        waveform_view_end_micros: waveform_key.waveform_view_end_micros,
        waveform_loop_enabled: waveform_key.waveform_loop_enabled,
        waveform_bpm_bits: waveform_key.waveform_bpm_bits,
        waveform_channel_view: waveform_key.waveform_channel_view,
        waveform_normalized_audition_enabled: waveform_key.waveform_normalized_audition_enabled,
        waveform_bpm_snap_enabled: waveform_key.waveform_bpm_snap_enabled,
        waveform_transient_snap_enabled: waveform_key.waveform_transient_snap_enabled,
        waveform_transient_markers_enabled: waveform_key.waveform_transient_markers_enabled,
        waveform_slice_mode_enabled: waveform_key.waveform_slice_mode_enabled,
        map_open: map_key.map_open,
        map_zoom_bits: map_key.map_zoom_bits,
        map_pan_x_bits: map_key.map_pan_x_bits,
        map_pan_y_bits: map_key.map_pan_y_bits,
        map_selection_revision: map_key.map_selection_revision,
        map_hover_revision: map_key.map_hover_revision,
        map_dataset_revision: map_key.map_dataset_revision,
        map_query_revision: map_key.map_query_revision,
        map_points_revision: map_key.map_points_revision,
        update_status: non_segment_static_key.update_status,
        update_revision: non_segment_static_key.update_revision,
        loaded_wav_revision: waveform_key.loaded_wav_revision,
        volume_milli: non_segment_static_key.volume_milli,
        transport_running: non_segment_static_key.transport_running,
        focus_context: non_segment_static_key.focus_context,
    }
}

/// Build a status-bar projection key from the current controller snapshot.
pub(super) fn build_status_projection_key(
    controller: &AppController,
    selected_column: usize,
) -> super::StatusProjectionCacheKey {
    status::build_status_projection_key(controller, selected_column)
}

/// Build a browser-frame projection key from the current controller snapshot.
pub(super) fn build_browser_frame_projection_key(
    controller: &AppController,
) -> super::BrowserFrameProjectionCacheKey {
    browser::build_browser_frame_projection_key(controller)
}

/// Build a browser-rows projection key from the current controller snapshot.
pub(super) fn build_browser_rows_projection_key(
    controller: &AppController,
) -> super::BrowserRowsProjectionCacheKey {
    browser::build_browser_rows_projection_key(controller)
}

/// Build a map-panel projection key from the current controller snapshot.
pub(super) fn build_map_projection_key(controller: &AppController) -> super::MapProjectionCacheKey {
    map::build_map_projection_key(controller)
}

/// Build a waveform projection key from the current controller snapshot.
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> super::WaveformProjectionCacheKey {
    waveform::build_waveform_projection_key(controller)
}

/// Build a projection key for static model fields outside explicit segment keys.
pub(super) fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> super::NonSegmentStaticProjectionCacheKey {
    non_segment::build_non_segment_static_projection_key(controller)
}
