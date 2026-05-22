//! Waveform projection DTOs for the Wavecrate native shell.

use super::NormalizedRangeModel;
use radiant::gui::visualization;

mod panel;

pub use self::panel::WaveformPanelModel;

/// Channel-view mode used by waveform rendering.
pub type WaveformChannelViewModel = visualization::ChannelViewMode;

/// Normalized waveform viewport state exposed to the native shell.
pub type WaveformViewportModel = visualization::TimelineViewport;

/// Waveform cursor, playhead, and selection transport state exposed to the native shell.
pub type WaveformTransportModel = visualization::TimelineTransportState;

/// Waveform edit selection and fade-preview state exposed to the native shell.
pub type WaveformEditPreviewModel = visualization::TimelineEditPreview;

/// One detected Wavecrate waveform slice preview exposed to the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformSlicePreviewModel {
    /// Slice range in normalized waveform precision.
    pub range: NormalizedRangeModel,
    /// Whether this slice is currently selected for edit operations.
    pub selected: bool,
    /// Whether this slice is focused for keyboard review.
    pub focused: bool,
    /// Whether this slice is marked for sample export.
    pub marked_for_export: bool,
    /// Whether this slice belongs to the duplicate-cleanup candidate batch.
    pub review_candidate: bool,
    /// Whether this slice is currently exempted from duplicate cleanup.
    pub review_exempted: bool,
}

/// One-shot waveform feedback event tokens exposed to the native shell.
pub type WaveformFeedbackEventsModel = visualization::TimelineFeedbackEvents;

/// Waveform guide/repeat/label presentation state exposed to the native shell.
pub type WaveformPresentationModel = visualization::TimelinePresentationState;

/// Retained waveform raster preview state exposed to the native shell.
pub type WaveformImagePreviewModel = visualization::SignalRasterPreview;

/// Waveform display chrome state exposed to the native shell.
pub type WaveformChromeStateModel = visualization::SignalChromeState;

/// Waveform tool availability state exposed to the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveformToolStateModel {
    /// Whether loop playback is locked against sample-driven updates.
    pub lock_enabled: bool,
    /// Whether normalized audition playback is enabled.
    pub audition_enabled: bool,
    /// Whether BPM snapping is enabled.
    pub primary_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_grid_enabled: bool,
    /// Whether transient snapping is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether transient markers are visible.
    pub markers_visible: bool,
    /// Whether slice review mode is active.
    pub review_mode_enabled: bool,
    /// Whether exact-duplicate cleanup can be applied from the waveform toolbar.
    pub cleanup_available: bool,
}

impl Default for WaveformToolStateModel {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            audition_enabled: false,
            primary_snap_enabled: false,
            relative_grid_enabled: false,
            secondary_snap_enabled: false,
            markers_visible: true,
            review_mode_enabled: false,
            cleanup_available: false,
        }
    }
}

impl WaveformToolStateModel {
    /// Build waveform tool state from explicit Wavecrate workflow flags.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        lock_enabled: bool,
        audition_enabled: bool,
        primary_snap_enabled: bool,
        relative_grid_enabled: bool,
        secondary_snap_enabled: bool,
        markers_visible: bool,
        review_mode_enabled: bool,
        cleanup_available: bool,
    ) -> Self {
        Self {
            lock_enabled,
            audition_enabled,
            primary_snap_enabled,
            relative_grid_enabled,
            secondary_snap_enabled,
            markers_visible,
            review_mode_enabled,
            cleanup_available,
        }
    }
}

/// Aggregated waveform timeline surface state exposed to the native shell.
pub type WaveformSurfaceModel = visualization::TimelineSurfaceState<WaveformSlicePreviewModel>;

/// Aggregated waveform motion state exposed to the native shell.
pub type WaveformMotionModel =
    visualization::TimelineMotionState<WaveformSlicePreviewModel, WaveformToolStateModel>;

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub compare_anchor_label: Option<String>,
    /// Whether loop state is locked against sample-driven auto-updates.
    pub loop_lock_enabled: bool,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
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
    pub exact_duplicate_cleanup_available: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            compare_anchor_available: false,
            compare_anchor_label: None,
            loop_lock_enabled: false,
            channel_view: WaveformChannelViewModel::Mono,
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
    pub fn signal_chrome(&self) -> WaveformChromeStateModel {
        WaveformChromeStateModel::new(
            self.transport_hint.clone(),
            self.compare_anchor_available,
            self.compare_anchor_label.clone(),
            self.channel_view,
        )
    }

    /// Return this chrome model's generic signal visualization tool state.
    pub fn signal_tools(&self) -> WaveformToolStateModel {
        WaveformToolStateModel::new(
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

/// Extract the numeric BPM portion from one projected tempo label.
pub fn parse_waveform_tempo_number_text(label: &str) -> Option<String> {
    let number = label.split_ascii_whitespace().next()?.trim();
    if number.is_empty() {
        return None;
    }
    let parsed = number.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(number.to_string())
}
