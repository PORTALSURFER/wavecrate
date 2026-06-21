mod bpm;
mod flash;
mod image;
mod playback;
mod selections;
mod slices;
mod transients;

pub use playback::{FadingPlayheadTrail, PlayheadSeek, PlayheadState, PlayheadTrailSample};
pub use selections::WaveformView;
pub use slices::{
    WaveformDuplicateCleanupPreview, WaveformDuplicateCleanupState, WaveformSliceBatchProfile,
    WaveformSliceReviewState,
};

use super::{UiPoint, controls::DestructiveEditPrompt};
use crate::selection::SelectionRange;
use crate::waveform::{WaveformChannelView, WaveformImage};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Cached waveform image and playback overlays.
#[derive(Clone, Debug)]
pub struct WaveformState {
    /// Cached rendered waveform image.
    pub image: Option<WaveformImage>,
    /// Producer-side waveform image identity used for projection/cache invalidation.
    pub waveform_image_signature: Option<u64>,
    /// Optional path for the sample currently loading to drive UI affordances.
    pub loading: Option<PathBuf>,

    /// Playhead position and trail state.
    pub playhead: PlayheadState,
    /// Last play start position chosen by the user (normalized 0-1).
    pub last_start_marker: Option<f32>,
    /// Whether looped playback is enabled.
    pub loop_enabled: bool,
    /// When true, loop playback state is locked against auto-updates.
    pub loop_lock_enabled: bool,
    /// Whether to normalize audition playback.
    pub normalized_audition_enabled: bool,

    /// Persistent navigation cursor (normalized 0-1) used by keyboard navigation.
    pub cursor: Option<f32>,
    /// Current selection range.
    pub selection: Option<SelectionRange>,
    /// Persisted playback-selection start used to anchor BPM grid rendering.
    ///
    /// This value is sample-local and survives selection clears so the BPM
    /// grid can remain aligned to the last established playmark origin until a
    /// new selection start replaces it. `0.0` represents the sample start
    /// before any playmark selection has been established.
    pub last_bpm_grid_origin: f32,
    /// Cached selection duration label.
    pub selection_duration: Option<String>,
    /// Optional edit selection range used for destructive edits (normalized 0-1).
    pub edit_selection: Option<SelectionRange>,
    /// Label showing the hovered time position.
    pub hover_time_label: Option<String>,
    /// Current waveform channel view mode.
    pub channel_view: WaveformChannelView,
    /// Current visible viewport within the waveform (0.0-1.0 normalized).
    pub view: WaveformView,
    /// Pending confirmation dialog for destructive edits.
    pub pending_destructive: Option<DestructiveEditPrompt>,
    /// Last moment the waveform cursor was moved via mouse hover.
    pub cursor_last_hover_at: Option<Instant>,
    /// Last moment the waveform cursor was moved via keyboard/navigation.
    pub cursor_last_navigation_at: Option<Instant>,
    /// Last pointer position seen over the waveform (screen space).
    pub hover_pointer_pos: Option<UiPoint>,
    /// Timestamp of the last time the pointer moved over the waveform.
    pub hover_pointer_last_moved_at: Option<Instant>,
    /// When true, hover should not override the cursor until the pointer moves.
    pub suppress_hover_cursor: bool,
    /// Last pointer position used for middle-button waveform panning.
    pub pan_drag_pos: Option<UiPoint>,

    /// Detected slice ranges for the current waveform.
    pub slices: Vec<SelectionRange>,
    /// Batch origin that determines how slice exports should be named.
    pub slice_batch_profile: WaveformSliceBatchProfile,
    /// Number of duplicate windows represented by the current exact-duplicate cleanup batch.
    ///
    /// This is only non-zero when `slice_batch_profile` is
    /// `WaveformSliceBatchProfile::ExactDuplicateBeats`.
    pub slice_batch_beat_count: usize,
    /// Exact-duplicate cleanup metadata for the current cleanup batch.
    ///
    /// This stays separate from generic slice review/export state so duplicate
    /// cleanup can track one preview per duplicate window plus user exemptions.
    pub duplicate_cleanup: Option<WaveformDuplicateCleanupState>,
    /// Indices of slice ranges currently selected for edits.
    pub selected_slices: Vec<usize>,
    /// Keyboard-first review state for previewed waveform slices.
    pub slice_review: WaveformSliceReviewState,
    /// When true, waveform drags paint slice ranges instead of selection.
    pub slice_mode_enabled: bool,

    /// When true, selection edits snap to beat-sized steps using the bpm value.
    pub bpm_snap_enabled: bool,
    /// When true, playback BPM grids and selection snapping anchor to the playmark selection.
    ///
    /// When false, the BPM grid and playback-selection snapping use the sample
    /// start (`0.0`) as their global anchor.
    pub relative_bpm_grid_enabled: bool,
    /// When true, loaded BPM metadata will not override the current BPM value.
    pub bpm_lock_enabled: bool,
    /// When true, loaded samples with BPM metadata are time-stretched to match the current BPM.
    pub bpm_stretch_enabled: bool,
    /// Last text input for the waveform BPM value.
    pub bpm_input: String,
    /// Parsed waveform BPM value used by snapping and stretching when valid.
    pub bpm_value: Option<f32>,

    /// Cached transient positions (normalized 0-1) for the loaded waveform.
    pub transients: Arc<[f32]>,
    /// When true, transient markers are rendered on the waveform.
    pub transient_markers_enabled: bool,
    /// When true, selection drags snap to nearby transient markers (disabled while hidden).
    pub transient_snap_enabled: bool,
    /// Cache token for the waveform transients.
    pub transient_cache_token: Option<u64>,

    /// Optional notice text displayed near the waveform.
    pub notice: Option<String>,
    /// User-facing compare-anchor label shown by waveform/transport compare controls.
    pub compare_anchor_label: Option<String>,

    /// Start time for the current waveform copy flash.
    pub copy_flash_at: Option<Instant>,
    /// Monotonic token incremented when a waveform selection export is queued.
    ///
    /// UI projections use this as a one-shot optimistic event marker so they
    /// can trigger immediate local blink feedback without depending on
    /// wall-clock synchronization with controller `Instant` values.
    pub selection_export_flash_nonce: u64,
    /// Monotonic token incremented when a queued waveform selection export fails.
    ///
    /// UI projections use this as a one-shot error event marker so they can
    /// repaint the selection with a stronger failure color after an optimistic
    /// submit flash has already been shown.
    pub selection_export_failure_flash_nonce: u64,
    /// Monotonic token incremented when preview edit effects are committed.
    ///
    /// UI projections use this as a one-shot success event marker so they can
    /// briefly brighten the edit-selection overlay when effect application
    /// succeeds without relying on synchronized wall-clock timestamps.
    pub edit_selection_apply_flash_nonce: u64,
}

impl Default for WaveformState {
    fn default() -> Self {
        let image = image::WaveformImageState::default();
        let playback = playback::WaveformPlaybackState::default();
        let selections = selections::WaveformSelectionState::default();
        let slices = slices::WaveformSlicesState::default();
        let bpm = bpm::WaveformBpmState::default();
        let transients = transients::WaveformTransientState::default();
        let flash = flash::WaveformFlashState::default();

        Self {
            image: image.image,
            waveform_image_signature: image.waveform_image_signature,
            loading: image.loading,
            playhead: playback.playhead,
            last_start_marker: playback.last_start_marker,
            loop_enabled: playback.loop_enabled,
            loop_lock_enabled: playback.loop_lock_enabled,
            normalized_audition_enabled: playback.normalized_audition_enabled,
            cursor: selections.cursor,
            selection: selections.selection,
            last_bpm_grid_origin: selections.last_bpm_grid_origin,
            selection_duration: selections.selection_duration,
            edit_selection: selections.edit_selection,
            hover_time_label: selections.hover_time_label,
            channel_view: selections.channel_view,
            view: selections.view,
            pending_destructive: selections.pending_destructive,
            cursor_last_hover_at: selections.cursor_last_hover_at,
            cursor_last_navigation_at: selections.cursor_last_navigation_at,
            hover_pointer_pos: selections.hover_pointer_pos,
            hover_pointer_last_moved_at: selections.hover_pointer_last_moved_at,
            suppress_hover_cursor: selections.suppress_hover_cursor,
            pan_drag_pos: selections.pan_drag_pos,
            slices: slices.ranges,
            slice_batch_profile: slices.batch_profile,
            slice_batch_beat_count: slices.batch_beat_count,
            duplicate_cleanup: slices.duplicate_cleanup,
            selected_slices: slices.selected_indices,
            slice_review: slices.review,
            slice_mode_enabled: slices.mode_enabled,
            bpm_snap_enabled: bpm.snap_enabled,
            relative_bpm_grid_enabled: bpm.relative_grid_enabled,
            bpm_lock_enabled: bpm.lock_enabled,
            bpm_stretch_enabled: bpm.stretch_enabled,
            bpm_input: bpm.input,
            bpm_value: bpm.value,
            transients: transients.positions,
            transient_markers_enabled: transients.markers_enabled,
            transient_snap_enabled: transients.snap_enabled,
            transient_cache_token: transients.cache_token,
            notice: None,
            compare_anchor_label: None,
            copy_flash_at: flash.copy_at,
            selection_export_flash_nonce: flash.selection_export_nonce,
            selection_export_failure_flash_nonce: flash.selection_export_failure_nonce,
            edit_selection_apply_flash_nonce: flash.edit_selection_apply_nonce,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WaveformSliceBatchProfile, WaveformState};

    #[test]
    fn waveform_state_defaults_without_image_signature() {
        let state = WaveformState::default();
        assert!(state.image.is_none());
        assert!(state.waveform_image_signature.is_none());
        assert_eq!(state.slice_batch_profile, WaveformSliceBatchProfile::Manual);
        assert_eq!(state.slice_batch_beat_count, 0);
        assert!(state.duplicate_cleanup.is_none());
        assert!(!state.slice_review.active);
        assert!(state.slice_review.focused_index.is_none());
        assert!(state.slice_review.marked_indices.is_empty());
    }
}
