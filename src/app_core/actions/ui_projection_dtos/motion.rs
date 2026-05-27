//! Motion-only UI projection DTOs.

mod projection;

use super::{AppModel, NormalizedRangeModel, WaveformChannelViewModel, WaveformSlicePreviewModel};

/// Motion-sensitive slice of the app model used for incremental overlay rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMotionModel {
    /// Transport animation state used by motion overlays.
    pub transport_running: bool,
    /// Whether map mode is active for tab overlay tinting.
    pub map_active: bool,
    /// Active browser rating-filter chip states for levels `-3..=3`, plus `4` for locked keeps.
    pub active_rating_filters: [bool; 8],
    /// Active browser playback-age filter chip states ordered as `Never`, `Month`, `Week`.
    pub active_playback_age_filters: [bool; 3],
    /// Whether the browser is currently filtering down to session-marked rows.
    pub marked_filter_active: bool,
    /// Waveform selected playback window with milli and micro precision.
    pub waveform_selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub waveform_slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub waveform_selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub waveform_selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    pub waveform_edit_selection_apply_flash_nonce: u64,
    /// Waveform edit-selection window with milli and micro precision.
    pub waveform_edit_selection_milli: Option<NormalizedRangeModel>,
    /// Waveform edit fade-in end handle in normalized milliseconds.
    pub waveform_edit_fade_in_end_milli: Option<u16>,
    /// Waveform edit fade-in end handle in normalized micro-units.
    pub waveform_edit_fade_in_end_micros: Option<u32>,
    /// Waveform edit fade-in mute-start handle in normalized milliseconds.
    pub waveform_edit_fade_in_mute_start_milli: Option<u16>,
    /// Waveform edit fade-in mute-start handle in normalized micro-units.
    pub waveform_edit_fade_in_mute_start_micros: Option<u32>,
    /// Waveform edit fade-in curve tension in normalized milliseconds.
    pub waveform_edit_fade_in_curve_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized milliseconds.
    pub waveform_edit_fade_out_start_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized micro-units.
    pub waveform_edit_fade_out_start_micros: Option<u32>,
    /// Waveform edit fade-out mute-end handle in normalized milliseconds.
    pub waveform_edit_fade_out_mute_end_milli: Option<u16>,
    /// Waveform edit fade-out mute-end handle in normalized micro-units.
    pub waveform_edit_fade_out_mute_end_micros: Option<u32>,
    /// Waveform edit fade-out curve tension in normalized milliseconds.
    pub waveform_edit_fade_out_curve_milli: Option<u16>,
    /// Whether loop playback is enabled for the active waveform selection.
    pub waveform_loop_enabled: bool,
    /// Whether loop playback is currently locked against sample-driven updates.
    pub waveform_loop_lock_enabled: bool,
    /// Waveform cursor position in normalized milliseconds.
    pub waveform_cursor_milli: Option<u16>,
    /// Waveform playhead position in normalized milliseconds.
    pub waveform_playhead_milli: Option<u16>,
    /// Waveform playhead position in normalized micro-units (`0..=1_000_000`).
    pub waveform_playhead_micros: Option<u32>,
    /// Current waveform view start in normalized milliseconds.
    pub waveform_view_start_milli: u16,
    /// Current waveform view end in normalized milliseconds.
    pub waveform_view_end_milli: u16,
    /// Current waveform view start in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_start_micros: u32,
    /// Current waveform view end in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_end_micros: u32,
    /// Current waveform view start in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Motion overlays use nanosecond bounds so rendered selection edges and
    /// playhead markers stay aligned with deep-zoom pointer geometry.
    pub waveform_view_start_nanos: u32,
    /// Current waveform view end in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Motion overlays use nanosecond bounds so rendered selection edges and
    /// playhead markers stay aligned with deep-zoom pointer geometry.
    pub waveform_view_end_nanos: u32,
    /// Human-readable tempo metadata.
    pub waveform_tempo_label: Option<String>,
    /// Human-readable zoom metadata.
    pub waveform_zoom_label: Option<String>,
    /// Loaded waveform label shown in the waveform overlay header.
    pub waveform_loaded_label: Option<String>,
    /// Whether the waveform plot is currently waiting for a new sample to load.
    pub waveform_loading: bool,
    /// Stable image signature for detecting waveform image updates during motion-only frames.
    pub waveform_image_signature: Option<u64>,
    /// Transport hint rendered with waveform metadata.
    pub waveform_transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub waveform_compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub waveform_compare_anchor_label: Option<String>,
    /// Current waveform channel-view mode.
    pub waveform_channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub waveform_normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled.
    pub waveform_bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub waveform_relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled.
    pub waveform_transient_snap_enabled: bool,
    /// Whether transient markers are visible.
    pub waveform_transient_markers_enabled: bool,
    /// Whether slice mode is active.
    pub waveform_slice_mode_enabled: bool,
    /// Whether exact-duplicate cleanup can be applied from the waveform toolbar.
    pub waveform_exact_duplicate_cleanup_available: bool,
    /// Right-aligned status-bar text rendered in the motion overlay.
    pub status_right: String,
}

impl NativeMotionModel {
    /// Build a motion model from a full application model snapshot.
    pub fn from_app_model(model: &AppModel) -> Self {
        let viewport = model.waveform.viewport();
        let transport = model.waveform.transport();
        let edit_preview = model.waveform.edit_preview();
        let feedback_events = model.waveform.feedback_events();
        let presentation = model.waveform.presentation();
        let image_preview = model.waveform.image_preview();
        let signal_chrome = model.waveform_chrome.signal_chrome();
        let signal_tools = model.waveform_chrome.signal_tools();

        Self {
            transport_running: model.transport_running,
            map_active: model.map.active,
            active_rating_filters: model.browser.active_rating_filters,
            active_playback_age_filters: model.browser.active_playback_age_filters,
            marked_filter_active: model.browser.marked_filter_active,
            waveform_selection_milli: transport.selection,
            waveform_slices: model.waveform.slices.clone(),
            waveform_selection_export_flash_nonce: feedback_events.primary_success_nonce,
            waveform_selection_export_failure_flash_nonce: feedback_events.primary_failure_nonce,
            waveform_edit_selection_apply_flash_nonce: feedback_events.secondary_success_nonce,
            waveform_edit_selection_milli: edit_preview.selection,
            waveform_edit_fade_in_end_milli: edit_preview.leading_end_milli,
            waveform_edit_fade_in_end_micros: edit_preview.leading_end_micros,
            waveform_edit_fade_in_mute_start_milli: edit_preview.leading_inner_start_milli,
            waveform_edit_fade_in_mute_start_micros: edit_preview.leading_inner_start_micros,
            waveform_edit_fade_in_curve_milli: edit_preview.leading_curve_milli,
            waveform_edit_fade_out_start_milli: edit_preview.trailing_start_milli,
            waveform_edit_fade_out_start_micros: edit_preview.trailing_start_micros,
            waveform_edit_fade_out_mute_end_milli: edit_preview.trailing_inner_end_milli,
            waveform_edit_fade_out_mute_end_micros: edit_preview.trailing_inner_end_micros,
            waveform_edit_fade_out_curve_milli: edit_preview.trailing_curve_milli,
            waveform_loop_enabled: presentation.repeat_enabled,
            waveform_loop_lock_enabled: signal_tools.lock_enabled,
            waveform_cursor_milli: transport.cursor_milli,
            waveform_playhead_milli: transport.playhead_milli,
            waveform_playhead_micros: transport.resolved_playhead_micros(),
            waveform_view_start_milli: viewport.start_milli,
            waveform_view_end_milli: viewport.end_milli,
            waveform_view_start_micros: viewport.start_micros,
            waveform_view_end_micros: viewport.end_micros,
            waveform_view_start_nanos: viewport.start_nanos,
            waveform_view_end_nanos: viewport.end_nanos,
            waveform_tempo_label: presentation.primary_label,
            waveform_zoom_label: presentation.viewport_label,
            waveform_loaded_label: image_preview.loaded_label,
            waveform_loading: image_preview.loading,
            waveform_image_signature: image_preview.image_signature,
            waveform_transport_hint: signal_chrome.status_hint,
            waveform_compare_anchor_available: signal_chrome.reference_anchor_available,
            waveform_compare_anchor_label: signal_chrome.reference_anchor_label,
            waveform_channel_view: signal_chrome.channel_view,
            waveform_normalized_audition_enabled: signal_tools.audition_enabled,
            waveform_bpm_snap_enabled: signal_tools.primary_snap_enabled,
            waveform_relative_bpm_grid_enabled: signal_tools.relative_grid_enabled,
            waveform_transient_snap_enabled: signal_tools.secondary_snap_enabled,
            waveform_transient_markers_enabled: signal_tools.markers_visible,
            waveform_slice_mode_enabled: signal_tools.review_mode_enabled,
            waveform_exact_duplicate_cleanup_available: signal_tools.cleanup_available,
            status_right: model.status.right.clone(),
        }
    }
}
