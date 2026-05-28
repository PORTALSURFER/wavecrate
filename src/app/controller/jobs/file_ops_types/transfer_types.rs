//! Transfer-oriented file operation DTOs for clipboard and drag/drop lanes.

use super::*;

/// Copy-vs-move mode for a background drop-target transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropTargetTransferKind {
    /// Duplicate the source file into the target folder.
    Copy,
    /// Relocate the source file into the target folder.
    Move,
}

impl DropTargetTransferKind {
    /// Human-readable past-tense verb for status reporting.
    pub(crate) const fn action_past_tense(self) -> &'static str {
        match self {
            Self::Copy => "Copied",
            Self::Move => "Moved",
        }
    }

    /// Human-readable present-participle verb for progress detail.
    pub(crate) const fn action_present_participle(self) -> &'static str {
        match self {
            Self::Copy => "Copying",
            Self::Move => "Moving",
        }
    }
}

/// Request payload for a background drop-target copy or move.
#[derive(Debug, Clone)]
pub(crate) struct DropTargetTransferRequest {
    /// Source identifier for the sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Relative path of the sample to transfer.
    pub(crate) relative_path: PathBuf,
    /// Cache-backed metadata captured on the controller when available.
    pub(crate) metadata: Option<DropTargetTransferMetadata>,
}

/// Metadata snapshot captured before a drop-target transfer starts.
#[derive(Debug, Clone)]
pub(crate) struct DropTargetTransferMetadata {
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
    /// Canonical sound classification, if any.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    /// Operator-authored custom tag, if any.
    pub(crate) user_tag: Option<String>,
    /// Normal library tag labels assigned to the sample.
    pub(crate) normal_tags: Vec<String>,
    /// Fixed collection slot assigned to the sample.
    pub(crate) collection: Option<crate::sample_sources::SampleCollection>,
}

/// Result of a background drop-target copy or move operation.
#[derive(Debug)]
pub(crate) struct DropTargetTransferResult {
    /// Transfer mode used for the batch.
    pub(crate) kind: DropTargetTransferKind,
    /// Target source identifier for the transfer.
    pub(crate) target_source_id: crate::sample_sources::SourceId,
    /// Human-readable label for the destination folder.
    pub(crate) target_label: String,
    /// Successfully transferred entries with metadata.
    pub(crate) transferred: Vec<DropTargetTransferSuccess>,
    /// Errors encountered during the transfer.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Record for a successfully copied or moved sample into a drop target.
#[derive(Debug)]
pub(crate) struct DropTargetTransferSuccess {
    /// Original source identifier.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original relative path.
    pub(crate) source_relative: PathBuf,
    /// New relative path at the destination.
    pub(crate) target_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
    /// Canonical sound classification, if any.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    /// Operator-authored custom tag, if any.
    pub(crate) user_tag: Option<String>,
    /// Normal library tag labels assigned to the sample.
    pub(crate) normal_tags: Vec<String>,
    /// Fixed collection slot assigned to the sample.
    pub(crate) collection: Option<crate::sample_sources::SampleCollection>,
}

/// Successful paste into a source folder with metadata for follow-up updates.
#[derive(Debug)]
pub(crate) struct SourcePasteAdded {
    /// Relative path of the added sample within the source root.
    pub(crate) relative_path: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
}

/// Result of pasting or importing files from the clipboard into a target.
#[derive(Debug)]
pub(crate) struct ClipboardPasteResult {
    /// Destination that received the pasted files.
    pub(crate) outcome: ClipboardPasteOutcome,
    /// Number of skipped files that were unsupported or missing.
    pub(crate) skipped: usize,
    /// Errors encountered while processing files.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
    /// Human-readable label for the target destination.
    pub(crate) target_label: String,
    /// Past-tense label for status reporting (e.g., "Pasted", "Imported").
    pub(crate) action_past_tense: &'static str,
}

/// Target-specific clipboard paste outcomes.
#[derive(Debug)]
pub(crate) enum ClipboardPasteOutcome {
    /// Paste into a source folder.
    Source {
        /// Source receiving the files.
        source_id: crate::sample_sources::SourceId,
        /// Added samples with metadata.
        added: Vec<SourcePasteAdded>,
    },
}

/// Request payload for a background source move operation.
#[derive(Debug, Clone)]
pub(crate) struct SourceMoveRequest {
    /// Source identifier for the sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Root folder for the source.
    pub(crate) source_root: PathBuf,
    /// Relative path of the sample to move.
    pub(crate) relative_path: PathBuf,
}

/// Result of a background source move operation.
#[derive(Debug)]
pub(crate) struct SourceMoveResult {
    /// Target source identifier for the move.
    pub(crate) target_source_id: crate::sample_sources::SourceId,
    /// Successful moves with metadata.
    pub(crate) moved: Vec<SourceMoveSuccess>,
    /// Errors encountered during the move.
    pub(crate) errors: Vec<String>,
    /// Whether the operation was cancelled by the user.
    pub(crate) cancelled: bool,
}

/// Record for a successfully moved sample.
#[derive(Debug)]
pub(crate) struct SourceMoveSuccess {
    /// Original source identifier.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Original relative path.
    pub(crate) relative_path: PathBuf,
    /// New relative path at the destination.
    pub(crate) target_relative: PathBuf,
    /// File size in bytes.
    pub(crate) file_size: u64,
    /// Modified time as epoch nanoseconds.
    pub(crate) modified_ns: i64,
    /// Tag associated with the sample.
    pub(crate) tag: crate::sample_sources::Rating,
    /// Loop marker state.
    pub(crate) looped: bool,
    /// Keep/lock marker state.
    pub(crate) locked: bool,
    /// Last played timestamp, if any.
    pub(crate) last_played_at: Option<i64>,
    /// Canonical sound classification, if any.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    /// Operator-authored custom tag, if any.
    pub(crate) user_tag: Option<String>,
    /// Normal library tag labels assigned to the sample.
    pub(crate) normal_tags: Vec<String>,
    /// Fixed collection slot assigned to the sample.
    pub(crate) collection: Option<crate::sample_sources::SampleCollection>,
}
