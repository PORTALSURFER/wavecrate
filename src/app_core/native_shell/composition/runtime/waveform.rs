//! Sempal waveform models used by the legacy Radiant compatibility path.

pub use crate::gui::range::NormalizedRange as NormalizedRangeModel;
use crate::gui::types::ImageRgba;
use std::sync::Arc;

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded content item, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused content item is still loading waveform data.
    pub loading: bool,
    /// Whether a replacement waveform image is still rendering in the background.
    pub image_rendering: bool,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units (`0..=1_000_000`).
    ///
    /// This preserves sub-milli transport precision for smooth playhead motion
    /// during animation-only redraws and full-model fallback rebuilds.
    pub playhead_micros: Option<u32>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub slices: Vec<crate::gui::visualization::TimelineMarkerPreview>,
    /// One-shot token incremented when a waveform-selection export is queued.
    ///
    /// Native shells treat each new value as an optimistic event and can run
    /// immediate local flash feedback without depending on controller
    /// wall-clock timestamps.
    pub selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    ///
    /// Native shells treat each new value as an error event so they can tint a
    /// later flash red after the initial optimistic feedback.
    pub selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    ///
    /// Native shells treat each new value as a success event so they can
    /// briefly brighten the edit-selection overlay after the write succeeds.
    pub edit_selection_apply_flash_nonce: u64,
    /// Current waveform edit-selection bounds (right-click paint range).
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    ///
    /// When absent, the fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_end_milli: Option<u16>,
    /// End position for the edit fade-in region in normalized micro-units.
    pub edit_fade_in_end_micros: Option<u32>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized micro-units.
    pub edit_fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    ///
    /// When absent, the fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_start_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized micro-units.
    pub edit_fade_out_start_micros: Option<u32>,
    /// End position for the edit fade-out mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_mute_end_milli: Option<u16>,
    /// End position for the edit fade-out mute region in normalized micro-units.
    pub edit_fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_out_curve_milli: Option<u16>,
    /// Visible view start in normalized milli-units.
    pub view_start_milli: u16,
    /// Visible view end in normalized milli-units.
    pub view_end_milli: u16,
    /// Visible view start in normalized micro-units (`0..=1_000_000`).
    pub view_start_micros: u32,
    /// Visible view end in normalized micro-units (`0..=1_000_000`).
    pub view_end_micros: u32,
    /// Visible view start in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Native input uses these fields for deep-zoom pointer-to-time mapping so
    /// click-to-play can preserve exact pixel intent even when the view span is
    /// narrower than one micro-unit.
    pub view_start_nanos: u32,
    /// Visible view end in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Native input uses these fields for deep-zoom pointer-to-time mapping so
    /// click-to-play can preserve exact pixel intent even when the view span is
    /// narrower than one micro-unit.
    pub view_end_nanos: u32,
    /// Quarter-note beat spacing in normalized micro-units when BPM/grid data is available.
    ///
    /// Native waveform renderers use this to draw a minor line on every beat
    /// and can accent every fourth beat as a bar boundary.
    pub beat_step_micros: Option<u32>,
    /// BPM grid origin in normalized micro-units.
    ///
    /// Native shells use this as the selection-relative anchor for drawing
    /// beat grid lines when no active selection is available. A value of `0`
    /// means no projected origin has been supplied yet.
    pub bpm_grid_origin_micros: u32,
    /// Whether loop playback is enabled.
    pub loop_enabled: bool,
    /// Optional tempo label rendered in waveform metadata.
    pub tempo_label: Option<String>,
    /// Optional zoom label rendered in waveform metadata.
    pub zoom_label: Option<String>,
    /// Cached signature for waveform image updates.
    pub waveform_image_signature: Option<u64>,
    /// Optional rasterized waveform payload for rendering the waveform preview.
    ///
    /// Hosts render this image inside the waveform plot area and keep overlays on top.
    /// The payload is shared so projection cache hits stay allocation-free.
    pub waveform_image: Option<Arc<ImageRgba>>,
}

impl Default for WaveformPanelModel {
    fn default() -> Self {
        Self {
            loaded_label: None,
            loading: false,
            image_rendering: false,
            cursor_milli: None,
            playhead_milli: None,
            playhead_micros: None,
            selection_milli: None,
            slices: Vec::new(),
            selection_export_flash_nonce: 0,
            selection_export_failure_flash_nonce: 0,
            edit_selection_apply_flash_nonce: 0,
            edit_selection_milli: None,
            edit_fade_in_end_milli: None,
            edit_fade_in_end_micros: None,
            edit_fade_in_mute_start_milli: None,
            edit_fade_in_mute_start_micros: None,
            edit_fade_in_curve_milli: None,
            edit_fade_out_start_milli: None,
            edit_fade_out_start_micros: None,
            edit_fade_out_mute_end_milli: None,
            edit_fade_out_mute_end_micros: None,
            edit_fade_out_curve_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            view_start_micros: 0,
            view_end_micros: 1_000_000,
            view_start_nanos: 0,
            view_end_nanos: 1_000_000_000,
            beat_step_micros: None,
            bpm_grid_origin_micros: 0,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

impl WaveformPanelModel {
    /// Return this panel's generic normalized timeline viewport.
    pub fn viewport(&self) -> crate::gui::visualization::TimelineViewport {
        crate::gui::visualization::TimelineViewport::new(
            self.view_start_milli,
            self.view_end_milli,
            self.view_start_micros,
            self.view_end_micros,
            self.view_start_nanos,
            self.view_end_nanos,
        )
    }

    /// Return this panel's generic timeline transport state.
    pub fn transport(&self) -> crate::gui::visualization::TimelineTransportState {
        crate::gui::visualization::TimelineTransportState::new(
            self.cursor_milli,
            self.playhead_milli,
            self.playhead_micros,
            self.selection_milli,
        )
    }

    /// Return this panel's generic timeline edit preview.
    pub fn edit_preview(&self) -> crate::gui::visualization::TimelineEditPreview {
        crate::gui::visualization::TimelineEditPreview::new(
            self.edit_selection_milli,
            self.edit_fade_in_end_milli,
            self.edit_fade_in_end_micros,
            self.edit_fade_in_mute_start_milli,
            self.edit_fade_in_mute_start_micros,
            self.edit_fade_in_curve_milli,
            self.edit_fade_out_start_milli,
            self.edit_fade_out_start_micros,
            self.edit_fade_out_mute_end_milli,
            self.edit_fade_out_mute_end_micros,
            self.edit_fade_out_curve_milli,
        )
    }

    /// Return this panel's generic timeline feedback events.
    pub fn feedback_events(&self) -> crate::gui::visualization::TimelineFeedbackEvents {
        crate::gui::visualization::TimelineFeedbackEvents::new(
            self.selection_export_flash_nonce,
            self.selection_export_failure_flash_nonce,
            self.edit_selection_apply_flash_nonce,
        )
    }

    /// Return this panel's generic timeline presentation state.
    pub fn presentation(&self) -> crate::gui::visualization::TimelinePresentationState {
        crate::gui::visualization::TimelinePresentationState::new(
            self.beat_step_micros,
            self.bpm_grid_origin_micros,
            self.loop_enabled,
            self.tempo_label.clone(),
            self.zoom_label.clone(),
        )
    }

    /// Return this panel's generic retained raster preview.
    pub fn image_preview(&self) -> crate::gui::visualization::SignalRasterPreview {
        crate::gui::visualization::SignalRasterPreview::new(
            self.loaded_label.clone(),
            self.loading,
            self.image_rendering,
            self.waveform_image_signature,
            self.waveform_image.clone(),
        )
    }

    /// Return this panel's generic normalized timeline surface state.
    pub fn timeline_surface(
        &self,
    ) -> crate::gui::visualization::TimelineSurfaceState<
        crate::gui::visualization::TimelineMarkerPreview,
    > {
        crate::gui::visualization::TimelineSurfaceState::new(
            self.viewport(),
            self.transport(),
            self.edit_preview(),
            self.feedback_events(),
            self.presentation(),
            self.image_preview(),
            self.slices.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformPanelModel;

    #[test]
    fn default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
    }

    #[test]
    fn viewport_projects_generic_timeline_bounds() {
        let model = WaveformPanelModel {
            view_start_milli: 250,
            view_end_milli: 500,
            view_start_micros: 250_000,
            view_end_micros: 500_000,
            view_start_nanos: 250_000_000,
            view_end_nanos: 500_000_000,
            ..WaveformPanelModel::default()
        };
        let viewport = model.viewport();

        assert_eq!(viewport.start_milli, 250);
        assert_eq!(viewport.end_milli, 500);
        assert_eq!(viewport.start_micros, 250_000);
        assert_eq!(viewport.end_micros, 500_000);
        assert_eq!(viewport.start_nanos, 250_000_000);
        assert_eq!(viewport.end_nanos, 500_000_000);
    }

    #[test]
    fn transport_projects_generic_timeline_positions() {
        let model = WaveformPanelModel {
            cursor_milli: Some(120),
            playhead_milli: Some(250),
            playhead_micros: None,
            selection_milli: Some(crate::gui::range::NormalizedRange::new(100, 400)),
            ..WaveformPanelModel::default()
        };
        let transport = model.transport();

        assert_eq!(transport.cursor_milli, Some(120));
        assert_eq!(transport.playhead_milli, Some(250));
        assert_eq!(transport.resolved_playhead_micros(), Some(250_000));
        assert_eq!(transport.selection, model.selection_milli);
    }

    #[test]
    fn edit_preview_projects_generic_timeline_handles() {
        let model = WaveformPanelModel {
            edit_selection_milli: Some(crate::gui::range::NormalizedRange::new(200, 800)),
            edit_fade_in_end_milli: Some(300),
            edit_fade_in_end_micros: Some(300_000),
            edit_fade_in_mute_start_milli: Some(240),
            edit_fade_in_mute_start_micros: Some(240_000),
            edit_fade_in_curve_milli: Some(420),
            edit_fade_out_start_milli: Some(700),
            edit_fade_out_start_micros: Some(700_000),
            edit_fade_out_mute_end_milli: Some(760),
            edit_fade_out_mute_end_micros: Some(760_000),
            edit_fade_out_curve_milli: Some(580),
            ..WaveformPanelModel::default()
        };
        let preview = model.edit_preview();

        assert_eq!(preview.selection, model.edit_selection_milli);
        assert_eq!(preview.leading_end_milli, Some(300));
        assert_eq!(preview.leading_inner_start_micros, Some(240_000));
        assert_eq!(preview.leading_curve_milli, Some(420));
        assert_eq!(preview.trailing_start_micros, Some(700_000));
        assert_eq!(preview.trailing_inner_end_milli, Some(760));
        assert_eq!(preview.trailing_curve_milli, Some(580));
    }

    #[test]
    fn feedback_events_project_generic_timeline_event_tokens() {
        let model = WaveformPanelModel {
            selection_export_flash_nonce: 7,
            selection_export_failure_flash_nonce: 8,
            edit_selection_apply_flash_nonce: 9,
            ..WaveformPanelModel::default()
        };
        let events = model.feedback_events();

        assert_eq!(events.primary_success_nonce, 7);
        assert_eq!(events.primary_failure_nonce, 8);
        assert_eq!(events.secondary_success_nonce, 9);
    }

    #[test]
    fn presentation_projects_generic_timeline_guides_repeat_and_labels() {
        let model = WaveformPanelModel {
            beat_step_micros: Some(125_000),
            bpm_grid_origin_micros: 25_000,
            loop_enabled: true,
            tempo_label: Some(String::from("120 BPM")),
            zoom_label: Some(String::from("4x")),
            ..WaveformPanelModel::default()
        };
        let presentation = model.presentation();

        assert_eq!(presentation.guide_step_micros, Some(125_000));
        assert_eq!(presentation.guide_origin_micros, 25_000);
        assert!(presentation.repeat_enabled);
        assert_eq!(presentation.primary_label.as_deref(), Some("120 BPM"));
        assert_eq!(presentation.viewport_label.as_deref(), Some("4x"));
    }

    #[test]
    fn image_preview_projects_generic_raster_state() {
        let image = std::sync::Arc::new(
            crate::gui::types::ImageRgba::new(1, 1, vec![0, 255, 0, 255]).unwrap(),
        );
        let model = WaveformPanelModel {
            loaded_label: Some(String::from("Loaded")),
            loading: true,
            image_rendering: false,
            waveform_image_signature: Some(99),
            waveform_image: Some(std::sync::Arc::clone(&image)),
            ..WaveformPanelModel::default()
        };
        let preview = model.image_preview();

        assert_eq!(preview.loaded_label.as_deref(), Some("Loaded"));
        assert!(preview.loading);
        assert!(!preview.image_rendering);
        assert_eq!(preview.image_signature, Some(99));
        assert_eq!(preview.image.as_deref(), Some(image.as_ref()));
    }

    #[test]
    fn timeline_surface_projects_generic_timeline_surface_state() {
        let model = WaveformPanelModel {
            view_start_micros: 125_000,
            playhead_micros: Some(250_250),
            selection_export_failure_flash_nonce: 5,
            loop_enabled: true,
            loaded_label: Some(String::from("Loaded")),
            slices: vec![crate::gui::visualization::TimelineMarkerPreview {
                range: crate::gui::range::NormalizedRange::new(100, 200),
                selected: true,
                focused: false,
                marked_for_export: false,
                duplicate_cleanup_candidate: false,
                duplicate_cleanup_exempted: false,
            }],
            ..WaveformPanelModel::default()
        };
        let surface = model.timeline_surface();

        assert_eq!(surface.viewport.start_micros, 125_000);
        assert_eq!(surface.transport.resolved_playhead_micros(), Some(250_250));
        assert_eq!(surface.feedback_events.primary_failure_nonce, 5);
        assert!(surface.presentation.repeat_enabled);
        assert_eq!(
            surface.raster_preview.loaded_label.as_deref(),
            Some("Loaded")
        );
        assert_eq!(surface.markers.len(), 1);
    }
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub compare_anchor_label: Option<String>,
    /// Whether loop state is locked against loaded-content auto-updates.
    pub loop_lock_enabled: bool,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: crate::gui::visualization::ChannelViewMode,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
    /// Whether the current slice batch is an exact-duplicate cleanup preview.
    ///
    /// Native shells use this to enable cleanup-only actions without exposing
    /// generic slice workflows such as silence-split export review.
    pub exact_duplicate_cleanup_available: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            compare_anchor_available: false,
            compare_anchor_label: None,
            loop_lock_enabled: false,
            channel_view: crate::gui::visualization::ChannelViewMode::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            relative_bpm_grid_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
            exact_duplicate_cleanup_available: false,
        }
    }
}

impl WaveformChromeModel {
    /// Return this chrome model's generic signal visualization display state.
    pub fn signal_chrome(&self) -> crate::gui::visualization::SignalChromeState {
        crate::gui::visualization::SignalChromeState::new(
            self.transport_hint.clone(),
            self.compare_anchor_available,
            self.compare_anchor_label.clone(),
            self.channel_view,
        )
    }

    /// Return this chrome model's generic signal visualization tool state.
    pub fn signal_tools(&self) -> crate::gui::visualization::SignalToolState {
        crate::gui::visualization::SignalToolState::new(
            self.loop_lock_enabled,
            self.normalized_audition_enabled,
            self.bpm_snap_enabled,
            self.relative_bpm_grid_enabled,
            self.transient_snap_enabled,
            self.transient_markers_enabled,
            self.slice_mode_enabled,
            self.exact_duplicate_cleanup_available,
        )
    }
}

#[cfg(test)]
mod chrome_tests {
    use super::WaveformChromeModel;

    #[test]
    fn signal_chrome_projects_generic_status_reference_and_channel_state() {
        let chrome = WaveformChromeModel {
            transport_hint: String::from("playing"),
            compare_anchor_available: true,
            compare_anchor_label: Some(String::from("A")),
            channel_view: crate::gui::visualization::ChannelViewMode::Stereo,
            ..WaveformChromeModel::default()
        };
        let signal_chrome = chrome.signal_chrome();

        assert_eq!(signal_chrome.status_hint, "playing");
        assert!(signal_chrome.reference_anchor_available);
        assert_eq!(signal_chrome.reference_anchor_label.as_deref(), Some("A"));
        assert_eq!(
            signal_chrome.channel_view,
            crate::gui::visualization::ChannelViewMode::Stereo
        );
    }

    #[test]
    fn signal_tools_project_generic_tool_flags() {
        let chrome = WaveformChromeModel {
            loop_lock_enabled: true,
            normalized_audition_enabled: true,
            bpm_snap_enabled: false,
            relative_bpm_grid_enabled: true,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: true,
            exact_duplicate_cleanup_available: false,
            ..WaveformChromeModel::default()
        };
        let tools = chrome.signal_tools();

        assert!(tools.lock_enabled);
        assert!(tools.audition_enabled);
        assert!(!tools.primary_snap_enabled);
        assert!(tools.relative_grid_enabled);
        assert!(!tools.secondary_snap_enabled);
        assert!(tools.markers_visible);
        assert!(tools.review_mode_enabled);
        assert!(!tools.cleanup_available);
    }
}
