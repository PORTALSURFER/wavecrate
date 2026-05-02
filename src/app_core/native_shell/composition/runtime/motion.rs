//! Sempal native-shell motion projection used by the legacy Radiant compatibility path.

use super::{AppModel, NormalizedRangeModel};

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
    pub waveform_slices: Vec<crate::gui::visualization::TimelineMarkerPreview>,
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
    /// Whether loop playback is currently locked against loaded-content updates.
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
    /// Whether the waveform plot is currently waiting for new content to load.
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
    pub waveform_channel_view: crate::gui::visualization::ChannelViewMode,
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
            active_playback_age_filters: model.browser.active_recency_filters,
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

    /// Return this motion snapshot's generic timeline viewport state.
    pub fn waveform_viewport(&self) -> crate::gui::visualization::TimelineViewport {
        crate::gui::visualization::TimelineViewport::new(
            self.waveform_view_start_milli,
            self.waveform_view_end_milli,
            self.waveform_view_start_micros,
            self.waveform_view_end_micros,
            self.waveform_view_start_nanos,
            self.waveform_view_end_nanos,
        )
    }

    /// Return this motion snapshot's generic timeline transport state.
    pub fn waveform_transport(&self) -> crate::gui::visualization::TimelineTransportState {
        crate::gui::visualization::TimelineTransportState::new(
            self.waveform_cursor_milli,
            self.waveform_playhead_milli,
            self.waveform_playhead_micros,
            self.waveform_selection_milli,
        )
    }

    /// Return this motion snapshot's generic timeline edit-preview state.
    pub fn waveform_edit_preview(&self) -> crate::gui::visualization::TimelineEditPreview {
        crate::gui::visualization::TimelineEditPreview::new(
            self.waveform_edit_selection_milli,
            self.waveform_edit_fade_in_end_milli,
            self.waveform_edit_fade_in_end_micros,
            self.waveform_edit_fade_in_mute_start_milli,
            self.waveform_edit_fade_in_mute_start_micros,
            self.waveform_edit_fade_in_curve_milli,
            self.waveform_edit_fade_out_start_milli,
            self.waveform_edit_fade_out_start_micros,
            self.waveform_edit_fade_out_mute_end_milli,
            self.waveform_edit_fade_out_mute_end_micros,
            self.waveform_edit_fade_out_curve_milli,
        )
    }

    /// Return this motion snapshot's generic timeline feedback event tokens.
    pub fn waveform_feedback_events(&self) -> crate::gui::visualization::TimelineFeedbackEvents {
        crate::gui::visualization::TimelineFeedbackEvents::new(
            self.waveform_selection_export_flash_nonce,
            self.waveform_selection_export_failure_flash_nonce,
            self.waveform_edit_selection_apply_flash_nonce,
        )
    }

    /// Return this motion snapshot's generic timeline presentation state.
    pub fn waveform_presentation(&self) -> crate::gui::visualization::TimelinePresentationState {
        crate::gui::visualization::TimelinePresentationState::new(
            None,
            0,
            self.waveform_loop_enabled,
            self.waveform_tempo_label.clone(),
            self.waveform_zoom_label.clone(),
        )
    }

    /// Return this motion snapshot's generic retained raster preview state.
    pub fn waveform_image_preview(&self) -> crate::gui::visualization::SignalRasterPreview {
        crate::gui::visualization::SignalRasterPreview::new(
            self.waveform_loaded_label.clone(),
            self.waveform_loading,
            false,
            self.waveform_image_signature,
            None,
        )
    }

    /// Return this motion snapshot's generic signal chrome state.
    pub fn signal_chrome(&self) -> crate::gui::visualization::SignalChromeState {
        crate::gui::visualization::SignalChromeState::new(
            self.waveform_transport_hint.clone(),
            self.waveform_compare_anchor_available,
            self.waveform_compare_anchor_label.clone(),
            self.waveform_channel_view,
        )
    }

    /// Return this motion snapshot's generic signal tool state.
    pub fn signal_tools(&self) -> crate::gui::visualization::SignalToolState {
        crate::gui::visualization::SignalToolState::new(
            self.waveform_loop_lock_enabled,
            self.waveform_normalized_audition_enabled,
            self.waveform_bpm_snap_enabled,
            self.waveform_relative_bpm_grid_enabled,
            self.waveform_transient_snap_enabled,
            self.waveform_transient_markers_enabled,
            self.waveform_slice_mode_enabled,
            self.waveform_exact_duplicate_cleanup_available,
        )
    }

    /// Return this motion snapshot as a generic timeline motion aggregate.
    pub fn timeline_motion(
        &self,
    ) -> crate::gui::visualization::TimelineMotionState<
        crate::gui::visualization::TimelineMarkerPreview,
    > {
        crate::gui::visualization::TimelineMotionState::new(
            self.transport_running,
            crate::gui::visualization::TimelineSurfaceState::new(
                self.waveform_viewport(),
                self.waveform_transport(),
                self.waveform_edit_preview(),
                self.waveform_feedback_events(),
                self.waveform_presentation(),
                self.waveform_image_preview(),
                self.waveform_slices.clone(),
            ),
            self.signal_chrome(),
            self.signal_tools(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeMotionModel, NormalizedRangeModel};

    #[test]
    fn native_motion_projects_generic_timeline_motion_state() {
        let model = NativeMotionModel {
            transport_running: true,
            map_active: false,
            active_rating_filters: [false; 8],
            active_playback_age_filters: [false; 3],
            marked_filter_active: false,
            waveform_selection_milli: Some(NormalizedRangeModel::new(100, 400)),
            waveform_slices: Vec::new(),
            waveform_selection_export_flash_nonce: 11,
            waveform_selection_export_failure_flash_nonce: 12,
            waveform_edit_selection_apply_flash_nonce: 13,
            waveform_edit_selection_milli: None,
            waveform_edit_fade_in_end_milli: Some(120),
            waveform_edit_fade_in_end_micros: Some(120_000),
            waveform_edit_fade_in_mute_start_milli: None,
            waveform_edit_fade_in_mute_start_micros: None,
            waveform_edit_fade_in_curve_milli: Some(200),
            waveform_edit_fade_out_start_milli: None,
            waveform_edit_fade_out_start_micros: None,
            waveform_edit_fade_out_mute_end_milli: Some(390),
            waveform_edit_fade_out_mute_end_micros: Some(390_000),
            waveform_edit_fade_out_curve_milli: Some(800),
            waveform_loop_enabled: true,
            waveform_loop_lock_enabled: true,
            waveform_cursor_milli: Some(150),
            waveform_playhead_milli: Some(250),
            waveform_playhead_micros: Some(250_500),
            waveform_view_start_milli: 10,
            waveform_view_end_milli: 900,
            waveform_view_start_micros: 10_000,
            waveform_view_end_micros: 900_000,
            waveform_view_start_nanos: 10_000_000,
            waveform_view_end_nanos: 900_000_000,
            waveform_tempo_label: Some(String::from("128 BPM")),
            waveform_zoom_label: Some(String::from("4x")),
            waveform_loaded_label: Some(String::from("Loaded")),
            waveform_loading: true,
            waveform_image_signature: Some(42),
            waveform_transport_hint: String::from("playing"),
            waveform_compare_anchor_available: true,
            waveform_compare_anchor_label: Some(String::from("A")),
            waveform_channel_view: crate::gui::visualization::ChannelViewMode::Stereo,
            waveform_normalized_audition_enabled: true,
            waveform_bpm_snap_enabled: true,
            waveform_relative_bpm_grid_enabled: false,
            waveform_transient_snap_enabled: true,
            waveform_transient_markers_enabled: true,
            waveform_slice_mode_enabled: false,
            waveform_exact_duplicate_cleanup_available: true,
            status_right: String::from("ready"),
        };

        let motion = model.timeline_motion();

        assert!(motion.transport_running);
        assert_eq!(motion.surface.viewport.start_micros, 10_000);
        assert_eq!(
            motion.surface.transport.resolved_playhead_micros(),
            Some(250_500)
        );
        assert_eq!(motion.surface.feedback_events.primary_success_nonce, 11);
        assert!(motion.surface.presentation.repeat_enabled);
        assert_eq!(
            motion.surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(motion.chrome.status_hint, "playing");
        assert_eq!(
            motion.chrome.channel_view,
            crate::gui::visualization::ChannelViewMode::Stereo
        );
        assert!(motion.tools.lock_enabled);
        assert!(motion.tools.cleanup_available);
    }
}
