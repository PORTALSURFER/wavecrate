//! Browser row DTOs for retained ui-projections.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Transient browser row processing states for batch file operations.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum BrowserRowProcessingState {
    /// The row is not part of an active row-scoped operation.
    #[default]
    None,
    /// The row is waiting in the current batch.
    Queued,
    /// The row is currently being processed.
    Active,
    /// The row completed successfully.
    Completed,
    /// The row was skipped by the batch.
    Skipped,
    /// The row failed during processing.
    Failed,
}

/// Visual playback-age buckets derived from sample playback history.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PlaybackAgeBucket {
    /// Samples played within the recent window, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
}

/// Browser playback-age filter chips shown in the native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlaybackAgeFilterChip {
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Summary of one Wavecrate browser/list row consumed by the UI projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRowModel {
    /// Visible row index in the filtered browser list.
    pub visible_row: usize,
    /// Display label for the row.
    ///
    /// This text is reference-counted so retained app-model clones can reuse
    /// row payloads without copying every row label.
    pub label: Arc<str>,
    /// Triage or grouping column index that currently owns the row.
    pub column: usize,
    /// Signed row rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Visual playback-age bucket used to render the row age marker.
    pub playback_age_bucket: PlaybackAgeBucket,
    /// Optional inline metadata label rendered at the row edge.
    pub bucket_label: Option<Arc<str>>,
    /// Optional normalized relatedness fill amount encoded in the inclusive `0..=255` range.
    pub similarity_display_strength: Option<u8>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing sample is unavailable.
    pub missing: bool,
    /// Whether the backing sample is locked/protected.
    pub locked: bool,
    /// Whether the backing sample is marked for later review.
    pub marked: bool,
    /// Transient row-scoped processing state for active batch file operations.
    pub processing_state: BrowserRowProcessingState,
}

impl BrowserRowModel {
    /// Build a row model, clamping the column into `0..=2`.
    pub fn new(
        visible_row: usize,
        label: impl Into<String>,
        column: usize,
        selected: bool,
        focused: bool,
    ) -> Self {
        Self {
            visible_row,
            label: Arc::<str>::from(label.into()),
            column: column.min(2),
            rating_level: 0,
            playback_age_bucket: PlaybackAgeBucket::Fresh,
            bucket_label: None,
            similarity_display_strength: None,
            selected,
            focused,
            missing: false,
            locked: false,
            marked: false,
            processing_state: BrowserRowProcessingState::None,
        }
    }

    /// Attach a signed rating level for inline row indicators.
    pub fn with_rating_level(mut self, rating_level: i8) -> Self {
        self.rating_level = rating_level.clamp(-3, 3);
        self
    }

    /// Attach the playback-age bucket used for row aging treatment.
    pub fn with_playback_age_bucket(mut self, playback_age_bucket: PlaybackAgeBucket) -> Self {
        self.playback_age_bucket = playback_age_bucket;
        self
    }

    /// Attach an explicit inline metadata label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(Arc::<str>::from(label.into()));
        self
    }

    /// Attach a normalized relatedness display strength for a compact row bar.
    ///
    /// Values are clamped into `[0.0, 1.0]` and encoded into the integer-backed
    /// `similarity_display_strength` field so retained app-model snapshots can
    /// keep `Eq` semantics.
    pub fn with_similarity_display_strength(mut self, display_strength: f32) -> Self {
        self.similarity_display_strength =
            Some(Self::encode_similarity_display_strength(display_strength));
        self
    }

    /// Encode one normalized relatedness display strength into the stored byte range.
    pub fn encode_similarity_display_strength(display_strength: f32) -> u8 {
        (display_strength.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Decode the stored relatedness display strength into a normalized fill amount.
    pub fn similarity_display_strength_ratio(&self) -> Option<f32> {
        self.similarity_display_strength
            .map(|strength| f32::from(strength) / 255.0)
    }

    /// Mark whether the backing sample is unavailable.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
        self
    }

    /// Mark whether the backing sample should render with protected treatment.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Mark whether the backing sample should render with review treatment.
    pub fn with_marked(mut self, marked: bool) -> Self {
        self.marked = marked;
        self
    }

    /// Attach a transient row-scoped processing state.
    pub fn with_processing_state(mut self, processing_state: BrowserRowProcessingState) -> Self {
        self.processing_state = processing_state;
        self
    }
}
