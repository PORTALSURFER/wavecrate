use crate::selection::SelectionRange;

pub(super) struct WaveformSlicesState {
    pub(super) ranges: Vec<SelectionRange>,
    pub(super) batch_profile: WaveformSliceBatchProfile,
    pub(super) batch_beat_count: usize,
    pub(super) duplicate_cleanup: Option<WaveformDuplicateCleanupState>,
    pub(super) selected_indices: Vec<usize>,
    pub(super) review: WaveformSliceReviewState,
    pub(super) mode_enabled: bool,
}

impl Default for WaveformSlicesState {
    fn default() -> Self {
        Self {
            ranges: Vec::new(),
            batch_profile: WaveformSliceBatchProfile::Manual,
            batch_beat_count: 0,
            duplicate_cleanup: None,
            selected_indices: Vec::new(),
            review: WaveformSliceReviewState::default(),
            mode_enabled: false,
        }
    }
}

/// Origin of the currently prepared waveform slice batch.
///
/// The controller uses this to keep export naming predictable for previewed
/// silence-split batches while preserving the existing manual slice naming
/// convention for user-authored slices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformSliceBatchProfile {
    /// Slice batch created manually or by retained slice tools.
    Manual,
    /// Slice batch created from silence-only detection.
    SilenceSplit,
    /// Slice batch created from exact BPM-aligned duplicate detection.
    ExactDuplicateBeats,
}

/// Reviewable duplicate-cleanup batch derived from one waveform scan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaveformDuplicateCleanupState {
    /// Number of duplicate groups that have one kept canonical hit plus clones.
    pub group_count: usize,
    /// Visible duplicate previews in original waveform order.
    pub previews: Vec<WaveformDuplicateCleanupPreview>,
}

/// One duplicate-cleanup preview entry shown in slice review mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformDuplicateCleanupPreview {
    /// Cleanup preview range in normalized waveform space.
    pub range: SelectionRange,
    /// Stable duplicate-group id for status text and future grouping logic.
    pub group_id: usize,
    /// Whether this preview is excluded from destructive cleanup.
    pub exempted: bool,
    /// Number of duplicate windows represented by this preview.
    ///
    /// This is `1` for detected windows and can be greater when the user merges
    /// multiple duplicate previews into one wider cleanup span.
    pub represented_window_count: usize,
}

/// Keyboard-oriented review state for previewed waveform slices.
///
/// Slice review intentionally stays separate from `selected_slices`: edit
/// selection still powers merge/delete flows, while review focus and export
/// marks drive fast audition/export decisions after silence splitting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaveformSliceReviewState {
    /// Whether keyboard slice review mode is currently active.
    pub active: bool,
    /// Zero-based index of the slice currently focused for audition.
    pub focused_index: Option<usize>,
    /// Zero-based slice indices explicitly marked for export.
    pub marked_indices: Vec<usize>,
}
