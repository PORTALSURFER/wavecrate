//! Waveform panel DTO and generic visualization projections.

use super::{
    NormalizedRangeModel, WaveformEditPreviewModel, WaveformFeedbackEventsModel,
    WaveformImagePreviewModel, WaveformPresentationModel, WaveformSlicePreviewModel,
    WaveformSurfaceModel, WaveformTransportModel, WaveformViewportModel,
};
use radiant::gui::types::ImageRgba;
use radiant::gui::visualization;
use std::sync::Arc;

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused sample is still loading waveform data.
    pub loading: bool,
    /// Whether a replacement waveform image is still rendering in the background.
    pub image_rendering: bool,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units (`0..=1_000_000`).
    pub playhead_micros: Option<u32>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    pub edit_selection_apply_flash_nonce: u64,
    /// Current waveform edit-selection bounds.
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    pub edit_fade_in_end_milli: Option<u16>,
    /// End position for the edit fade-in region in normalized micro-units.
    pub edit_fade_in_end_micros: Option<u32>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized micro-units.
    pub edit_fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    pub edit_fade_out_start_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized micro-units.
    pub edit_fade_out_start_micros: Option<u32>,
    /// End position for the edit fade-out mute region in normalized milli-units.
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
    pub view_start_nanos: u32,
    /// Visible view end in normalized nanounits (`0..=1_000_000_000`).
    pub view_end_nanos: u32,
    /// Quarter-note beat spacing in normalized micro-units when BPM/grid data is available.
    pub beat_step_micros: Option<u32>,
    /// BPM grid origin in normalized micro-units.
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
    pub fn viewport(&self) -> WaveformViewportModel {
        WaveformViewportModel::new(
            self.view_start_milli,
            self.view_end_milli,
            self.view_start_micros,
            self.view_end_micros,
            self.view_start_nanos,
            self.view_end_nanos,
        )
    }

    /// Return this panel's generic timeline transport state.
    pub fn transport(&self) -> WaveformTransportModel {
        WaveformTransportModel::new(
            self.cursor_milli,
            self.playhead_milli,
            self.playhead_micros,
            self.selection_milli,
        )
    }

    /// Return this panel's generic timeline edit preview.
    pub fn edit_preview(&self) -> WaveformEditPreviewModel {
        WaveformEditPreviewModel::from_parts(visualization::TimelineEditPreviewParts {
            selection: self.edit_selection_milli,
            leading_end_milli: self.edit_fade_in_end_milli,
            leading_end_micros: self.edit_fade_in_end_micros,
            leading_inner_start_milli: self.edit_fade_in_mute_start_milli,
            leading_inner_start_micros: self.edit_fade_in_mute_start_micros,
            leading_curve_milli: self.edit_fade_in_curve_milli,
            trailing_start_milli: self.edit_fade_out_start_milli,
            trailing_start_micros: self.edit_fade_out_start_micros,
            trailing_inner_end_milli: self.edit_fade_out_mute_end_milli,
            trailing_inner_end_micros: self.edit_fade_out_mute_end_micros,
            trailing_curve_milli: self.edit_fade_out_curve_milli,
        })
    }

    /// Return this panel's generic timeline feedback events.
    pub fn feedback_events(&self) -> WaveformFeedbackEventsModel {
        WaveformFeedbackEventsModel::new(
            self.selection_export_flash_nonce,
            self.selection_export_failure_flash_nonce,
            self.edit_selection_apply_flash_nonce,
        )
    }

    /// Return this panel's generic timeline presentation state.
    pub fn presentation(&self) -> WaveformPresentationModel {
        WaveformPresentationModel::new(
            self.beat_step_micros,
            self.bpm_grid_origin_micros,
            self.loop_enabled,
            self.tempo_label.clone(),
            self.zoom_label.clone(),
        )
    }

    /// Return this panel's generic retained raster preview.
    pub fn image_preview(&self) -> WaveformImagePreviewModel {
        WaveformImagePreviewModel::new(
            self.loaded_label.clone(),
            self.loading,
            self.image_rendering,
            self.waveform_image_signature,
            self.waveform_image.clone(),
        )
    }

    /// Return this panel's generic normalized timeline surface state.
    pub fn timeline_surface(&self) -> WaveformSurfaceModel {
        WaveformSurfaceModel::from_parts(visualization::TimelineSurfaceParts {
            viewport: self.viewport(),
            transport: self.transport(),
            edit_preview: self.edit_preview(),
            feedback_events: self.feedback_events(),
            presentation: self.presentation(),
            raster_preview: self.image_preview(),
            markers: self.slices.clone(),
        })
    }
}
