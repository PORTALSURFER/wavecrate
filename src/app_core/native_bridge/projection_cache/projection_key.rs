use super::super::projection_key_encoding::{
    encode_browser_filter, encode_browser_sort, encode_browser_tab, encode_focus_context,
    encode_update_status, normalized_f32_to_milli, normalized_f64_to_milli,
};
use super::{
    BrowserFrameProjectionCacheKey, BrowserRowsProjectionCacheKey, MapProjectionCacheKey,
    NativeProjectionCacheKey, NonSegmentStaticProjectionCacheKey, StatusProjectionCacheKey,
    WaveformProjectionCacheKey,
};
use crate::app_core::controller::AppController;

/// Build the full projection cache key from current controller state.
pub(super) fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    let waveform_millis = derive_waveform_projection_millis(controller);
    NativeProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_filter: encode_browser_filter(controller.ui.browser.filter),
        browser_sort: encode_browser_sort(controller.ui.browser.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        progress_visible: controller.ui.progress.visible,
        progress_completed: controller.ui.progress.completed,
        progress_total: controller.ui.progress.total,
        prompt_active: controller.ui.browser.pending_action.is_some()
            || controller.ui.sources.folders.pending_action.is_some()
            || controller.ui.sources.folders.new_folder.is_some()
            || controller.ui.waveform.pending_destructive.is_some(),
        drag_active: controller.ui.drag.payload.is_some(),
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_selection_start_milli: waveform_millis.selection_start_milli,
        waveform_selection_end_milli: waveform_millis.selection_end_milli,
        waveform_edit_selection_start_milli: waveform_millis.edit_selection_start_milli,
        waveform_edit_selection_end_milli: waveform_millis.edit_selection_end_milli,
        waveform_edit_fade_in_end_milli: waveform_millis.edit_fade_in_end_milli,
        waveform_edit_fade_in_mute_start_milli: waveform_millis.edit_fade_in_mute_start_milli,
        waveform_edit_fade_in_curve_milli: waveform_millis.edit_fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: waveform_millis.edit_fade_out_start_milli,
        waveform_edit_fade_out_mute_end_milli: waveform_millis.edit_fade_out_mute_end_milli,
        waveform_edit_fade_out_curve_milli: waveform_millis.edit_fade_out_curve_milli,
        waveform_view_start_milli: waveform_millis.view_start_milli,
        waveform_view_end_milli: waveform_millis.view_end_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        waveform_channel_view: encode_waveform_channel_view(controller),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selection_revision: controller.ui.projection_revisions.map_selection,
        map_hover_revision: controller.ui.projection_revisions.map_hover,
        map_dataset_revision: controller.ui.projection_revisions.map_dataset,
        map_query_revision: controller.ui.projection_revisions.map_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
        focus_context: encode_focus_context(controller.ui.focus.context),
    }
}

/// Build a status-bar projection key from the current controller snapshot.
pub(super) fn build_status_projection_key(
    controller: &AppController,
    selected_column: usize,
) -> StatusProjectionCacheKey {
    StatusProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search_busy,
        selected_column,
    }
}

/// Build a browser-frame projection key from the current controller snapshot.
pub(super) fn build_browser_frame_projection_key(
    controller: &AppController,
) -> BrowserFrameProjectionCacheKey {
    BrowserFrameProjectionCacheKey {
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search_busy,
        browser_sort: encode_browser_sort(controller.ui.browser.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        browser_similarity_follow_loaded: controller.ui.browser.similarity_sort_follow_loaded,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
    }
}

/// Build a browser-rows projection key from the current controller snapshot.
pub(super) fn build_browser_rows_projection_key(
    controller: &AppController,
) -> BrowserRowsProjectionCacheKey {
    BrowserRowsProjectionCacheKey {
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}

/// Build a map-panel projection key from the current controller snapshot.
pub(super) fn build_map_projection_key(controller: &AppController) -> MapProjectionCacheKey {
    MapProjectionCacheKey {
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selection_revision: controller.ui.projection_revisions.map_selection,
        map_hover_revision: controller.ui.projection_revisions.map_hover,
        map_dataset_revision: controller.ui.projection_revisions.map_dataset,
        map_query_revision: controller.ui.projection_revisions.map_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}

/// Build a waveform projection key from the current controller snapshot.
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> WaveformProjectionCacheKey {
    let waveform_millis = derive_waveform_projection_millis(controller);
    WaveformProjectionCacheKey {
        waveform_signature: controller.ui.waveform.waveform_image_signature,
        waveform_selection_start_milli: waveform_millis.selection_start_milli,
        waveform_selection_end_milli: waveform_millis.selection_end_milli,
        waveform_edit_selection_start_milli: waveform_millis.edit_selection_start_milli,
        waveform_edit_selection_end_milli: waveform_millis.edit_selection_end_milli,
        waveform_edit_fade_in_end_milli: waveform_millis.edit_fade_in_end_milli,
        waveform_edit_fade_in_mute_start_milli: waveform_millis.edit_fade_in_mute_start_milli,
        waveform_edit_fade_in_curve_milli: waveform_millis.edit_fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: waveform_millis.edit_fade_out_start_milli,
        waveform_edit_fade_out_mute_end_milli: waveform_millis.edit_fade_out_mute_end_milli,
        waveform_edit_fade_out_curve_milli: waveform_millis.edit_fade_out_curve_milli,
        waveform_view_start_milli: waveform_millis.view_start_milli,
        waveform_view_end_milli: waveform_millis.view_end_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_bpm_bits: controller.ui.waveform.bpm_value.map(f32::to_bits),
        waveform_channel_view: encode_waveform_channel_view(controller),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
        transport_running: controller.is_playing(),
    }
}

/// Build a projection key for static model fields outside explicit segment keys.
pub(super) fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
        focus_context: encode_focus_context(controller.ui.focus.context),
        trash_count: controller.ui.browser.trash.len(),
        neutral_count: controller.ui.browser.neutral.len(),
        keep_count: controller.ui.browser.keep.len(),
    }
}

/// Normalized waveform projection values converted to milli-space key fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WaveformProjectionMillis {
    selection_start_milli: Option<u16>,
    selection_end_milli: Option<u16>,
    edit_selection_start_milli: Option<u16>,
    edit_selection_end_milli: Option<u16>,
    edit_fade_in_end_milli: Option<u16>,
    edit_fade_in_mute_start_milli: Option<u16>,
    edit_fade_in_curve_milli: Option<u16>,
    edit_fade_out_start_milli: Option<u16>,
    edit_fade_out_mute_end_milli: Option<u16>,
    edit_fade_out_curve_milli: Option<u16>,
    view_start_milli: u16,
    view_end_milli: u16,
}

/// Derive normalized waveform projection key fields once for cache-key builders.
fn derive_waveform_projection_millis(controller: &AppController) -> WaveformProjectionMillis {
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
    ) = controller
        .ui
        .waveform
        .edit_selection
        .map(|selection| {
            let start = selection.start();
            let end = selection.end();
            let width = selection.width();
            if width <= 0.0 {
                return (None, None, None, None);
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
            (
                fade_in_end,
                fade_in_mute_start,
                fade_out_start,
                fade_out_mute_end,
            )
        })
        .unwrap_or((None, None, None, None));
    WaveformProjectionMillis {
        selection_start_milli,
        selection_end_milli,
        edit_selection_start_milli,
        edit_selection_end_milli,
        edit_fade_in_end_milli,
        edit_fade_in_mute_start_milli,
        edit_fade_in_curve_milli,
        edit_fade_out_start_milli,
        edit_fade_out_mute_end_milli,
        edit_fade_out_curve_milli,
        view_start_milli: normalized_f64_to_milli(controller.ui.waveform.view.start),
        view_end_milli: normalized_f64_to_milli(controller.ui.waveform.view.end),
    }
}

/// Encode waveform channel-view mode for compact projection keys.
fn encode_waveform_channel_view(controller: &AppController) -> u8 {
    match controller.ui.waveform.channel_view {
        crate::waveform::WaveformChannelView::Mono => 0,
        crate::waveform::WaveformChannelView::SplitStereo => 1,
    }
}
