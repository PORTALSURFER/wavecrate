//! Browser-facing file operation DTOs for sample and folder mutation results.

use super::*;

/// Result of deleting one browser sample file in the background.
#[derive(Debug)]
pub(crate) struct SampleDeleteResult {
    /// Source that owned the deleted samples.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Relative sample paths that were requested for deletion.
    pub(crate) requested_paths: Vec<PathBuf>,
    /// Relative sample paths that were deleted successfully.
    pub(crate) deleted_paths: Vec<PathBuf>,
    /// Follow-up browser focus plan captured before deletion.
    pub(crate) next_focus: Option<
        crate::app::controller::library::browser_controller::helpers::DeleteBrowserFocusPlan,
    >,
    /// Final error reported by the worker when any deletion failed.
    pub(crate) last_error: Option<String>,
}

/// Result of renaming one browser sample file in the background.
#[derive(Debug)]
pub(crate) struct SampleRenameResult {
    /// Source that owned the renamed sample.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Previous relative sample path.
    pub(crate) old_relative: PathBuf,
    /// New relative sample path.
    pub(crate) new_relative: PathBuf,
    /// Updated entry metadata aligned to the renamed file.
    pub(crate) entry: Option<crate::sample_sources::WavEntry>,
    /// Whether playback should resume once the renamed sample reloads.
    pub(crate) resume_playback: bool,
    /// Loop state that should be restored for playback resume.
    pub(crate) resume_looped: bool,
    /// Optional playback start override to restore after rename.
    pub(crate) resume_start_override: Option<f64>,
    /// Terminal rename outcome.
    pub(crate) result: Result<(), String>,
}

/// One successful sample auto-rename entry in a batch.
#[derive(Debug)]
pub(crate) struct SampleAutoRenameSuccess {
    /// Previous relative sample path.
    pub(crate) old_relative: PathBuf,
    /// New relative sample path.
    pub(crate) new_relative: PathBuf,
    /// Updated entry metadata aligned to the renamed file.
    pub(crate) entry: crate::sample_sources::WavEntry,
    /// Whether playback should resume once the renamed sample reloads.
    pub(crate) resume_playback: bool,
    /// Loop state that should be restored for playback resume.
    pub(crate) resume_looped: bool,
    /// Optional playback start override to restore after rename.
    pub(crate) resume_start_override: Option<f64>,
}

/// Result of running auto-rename across one browser selection batch.
#[derive(Debug)]
pub(crate) struct SampleAutoRenameResult {
    /// Source that owned the selected samples.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Relative sample paths requested for auto rename.
    pub(crate) requested_paths: Vec<PathBuf>,
    /// Successfully renamed samples.
    pub(crate) renamed: Vec<SampleAutoRenameSuccess>,
    /// Samples skipped before rename plus the user-facing reason.
    pub(crate) skipped: Vec<(PathBuf, String)>,
    /// Fatal rename failures encountered during the batch.
    pub(crate) errors: Vec<(PathBuf, String)>,
}

/// Structured per-item auto-rename progress emitted by the file-op worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SampleAutoRenameProgress {
    /// Worker started processing one requested path.
    Active { old_relative: PathBuf },
    /// Worker completed one requested path and may have remapped it.
    Completed {
        old_relative: PathBuf,
        new_relative: PathBuf,
    },
    /// Worker skipped one requested path without applying a rename.
    Skipped {
        old_relative: PathBuf,
        reason: String,
    },
    /// Worker failed one requested path.
    Failed {
        old_relative: PathBuf,
        error: String,
    },
}

/// Result of creating one folder in the background.
#[derive(Debug)]
pub(crate) struct FolderCreateResult {
    /// Source that owns the created folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Relative folder path that was created.
    pub(crate) relative_path: PathBuf,
    /// Terminal creation outcome.
    pub(crate) result: Result<(), String>,
}

/// Result of renaming one folder in the background.
#[derive(Debug)]
pub(crate) struct FolderRenameResult {
    /// Source that owns the renamed folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Previous relative folder path.
    pub(crate) old_folder: PathBuf,
    /// New relative folder path.
    pub(crate) new_folder: PathBuf,
    /// Updated sample entries aligned to the renamed folder paths.
    pub(crate) entries: Vec<crate::sample_sources::WavEntry>,
    /// Terminal rename outcome.
    pub(crate) result: Result<(), String>,
}

/// Result of deleting one folder in the background.
#[derive(Debug)]
pub(crate) struct FolderDeleteResult {
    /// Source that owned the deleted folder.
    pub(crate) source_id: crate::sample_sources::SourceId,
    /// Source root used for undo and recovery application.
    pub(crate) source_root: PathBuf,
    /// Deleted folder path relative to the source root.
    pub(crate) relative_path: PathBuf,
    /// Folder entries removed alongside the folder delete.
    pub(crate) entries: Vec<crate::sample_sources::WavEntry>,
    /// Staging root that retains the deleted folder for recovery.
    pub(crate) staging_root: PathBuf,
    /// Delete staging info returned by the recovery layer.
    pub(crate) staged:
        Option<crate::app::controller::library::source_folders::delete_recovery::DeleteStagingInfo>,
    /// Optional next-focused folder path captured before deletion.
    pub(crate) next_focus: Option<PathBuf>,
    /// Terminal delete outcome.
    pub(crate) result: Result<(), String>,
}
