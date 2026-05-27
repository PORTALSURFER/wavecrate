//! Generic waveform projection helpers for motion-only snapshots.

use radiant::gui::visualization;

use super::super::{
    WaveformChromeStateModel, WaveformEditPreviewModel, WaveformFeedbackEventsModel,
    WaveformImagePreviewModel, WaveformMotionModel, WaveformPresentationModel,
    WaveformSurfaceModel, WaveformToolStateModel, WaveformTransportModel, WaveformViewportModel,
};
use super::NativeMotionModel;

impl NativeMotionModel {
    /// Return this motion snapshot's generic timeline viewport state.
    pub fn waveform_viewport(&self) -> WaveformViewportModel {
        WaveformViewportModel::new(
            self.waveform_view_start_milli,
            self.waveform_view_end_milli,
            self.waveform_view_start_micros,
            self.waveform_view_end_micros,
            self.waveform_view_start_nanos,
            self.waveform_view_end_nanos,
        )
    }

    /// Return this motion snapshot's generic timeline transport state.
    pub fn waveform_transport(&self) -> WaveformTransportModel {
        WaveformTransportModel::new(
            self.waveform_cursor_milli,
            self.waveform_playhead_milli,
            self.waveform_playhead_micros,
            self.waveform_selection_milli,
        )
    }

    /// Return this motion snapshot's generic timeline edit-preview state.
    pub fn waveform_edit_preview(&self) -> WaveformEditPreviewModel {
        WaveformEditPreviewModel::from_parts(visualization::TimelineEditPreviewParts {
            selection: self.waveform_edit_selection_milli,
            leading_end_milli: self.waveform_edit_fade_in_end_milli,
            leading_end_micros: self.waveform_edit_fade_in_end_micros,
            leading_inner_start_milli: self.waveform_edit_fade_in_mute_start_milli,
            leading_inner_start_micros: self.waveform_edit_fade_in_mute_start_micros,
            leading_curve_milli: self.waveform_edit_fade_in_curve_milli,
            trailing_start_milli: self.waveform_edit_fade_out_start_milli,
            trailing_start_micros: self.waveform_edit_fade_out_start_micros,
            trailing_inner_end_milli: self.waveform_edit_fade_out_mute_end_milli,
            trailing_inner_end_micros: self.waveform_edit_fade_out_mute_end_micros,
            trailing_curve_milli: self.waveform_edit_fade_out_curve_milli,
        })
    }

    /// Return this motion snapshot's generic timeline feedback event tokens.
    pub fn waveform_feedback_events(&self) -> WaveformFeedbackEventsModel {
        WaveformFeedbackEventsModel::new(
            self.waveform_selection_export_flash_nonce,
            self.waveform_selection_export_failure_flash_nonce,
            self.waveform_edit_selection_apply_flash_nonce,
        )
    }

    /// Return this motion snapshot's generic timeline presentation state.
    pub fn waveform_presentation(&self) -> WaveformPresentationModel {
        WaveformPresentationModel::new(
            None,
            0,
            self.waveform_loop_enabled,
            self.waveform_tempo_label.clone(),
            self.waveform_zoom_label.clone(),
        )
    }

    /// Return this motion snapshot's generic retained raster preview state.
    pub fn waveform_image_preview(&self) -> WaveformImagePreviewModel {
        WaveformImagePreviewModel::new(
            self.waveform_loaded_label.clone(),
            self.waveform_loading,
            false,
            self.waveform_image_signature,
            None,
        )
    }

    /// Return this motion snapshot's generic signal chrome state.
    pub fn signal_chrome(&self) -> WaveformChromeStateModel {
        WaveformChromeStateModel::new(
            self.waveform_transport_hint.clone(),
            self.waveform_compare_anchor_available,
            self.waveform_compare_anchor_label.clone(),
            self.waveform_channel_view,
        )
    }

    /// Return this motion snapshot's generic signal tool state.
    pub fn signal_tools(&self) -> WaveformToolStateModel {
        WaveformToolStateModel::new(
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
    pub fn timeline_motion(&self) -> WaveformMotionModel {
        WaveformMotionModel::new(
            self.transport_running,
            WaveformSurfaceModel::from_parts(visualization::TimelineSurfaceParts {
                viewport: self.waveform_viewport(),
                transport: self.waveform_transport(),
                edit_preview: self.waveform_edit_preview(),
                feedback_events: self.waveform_feedback_events(),
                presentation: self.waveform_presentation(),
                raster_preview: self.waveform_image_preview(),
                markers: self.waveform_slices.clone(),
            }),
            self.signal_chrome(),
            self.signal_tools(),
        )
    }
}
