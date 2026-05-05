//! Destructive edit result DTOs for background waveform/file mutations.

use super::*;

/// Result of one background destructive selection edit commit.
#[derive(Debug)]
pub(crate) struct SelectionEditCommitResult {
    /// Source that owns the edited sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Edited sample path relative to the source root.
    pub(crate) relative_path: PathBuf,
    /// Absolute path overwritten by the worker.
    pub(crate) absolute_path: PathBuf,
    /// Human-readable undo label and status prefix.
    pub(crate) action_label: String,
    /// Status message to show after the commit succeeds.
    pub(crate) status_message: String,
    /// Whether the pre-edit selection/view should be restored after apply.
    pub(crate) preserve_selection: bool,
    /// Visual state snapshot captured before the edit started.
    pub(crate) visual: crate::app::controller::library::selection_edits::SelectionEditVisualState,
    /// Playback state captured before the edit started.
    pub(crate) playback: crate::app::controller::library::selection_edits::PlaybackResumeState,
    /// Whether the edit should clear duplicate-cleanup overlays after apply.
    pub(crate) clear_duplicate_cleanup: bool,
    /// Whether the edit should clear preview fade handles and flash the edit selection.
    pub(crate) clear_edit_fades: bool,
    /// Updated entry metadata for the overwritten sample.
    pub(crate) entry: Option<crate::sample_sources::WavEntry>,
    /// Deferred undo backup captured off the UI thread.
    pub(crate) backup: Option<crate::app::controller::undo::OverwriteBackup>,
    /// Terminal commit outcome.
    pub(crate) result: Result<(), String>,
}

/// Result of one background circular-slide overwrite commit.
#[derive(Debug)]
pub(crate) struct WaveformSlideCommitResult {
    /// Source that owns the edited sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Edited sample path relative to the source root.
    pub(crate) relative_path: PathBuf,
    /// Absolute path overwritten by the worker.
    pub(crate) absolute_path: PathBuf,
    /// Updated entry metadata for the overwritten sample.
    pub(crate) entry: Option<crate::sample_sources::WavEntry>,
    /// Deferred undo backup captured off the UI thread.
    pub(crate) backup: Option<crate::app::controller::undo::OverwriteBackup>,
    /// Terminal slide outcome.
    pub(crate) result: Result<(), String>,
}
