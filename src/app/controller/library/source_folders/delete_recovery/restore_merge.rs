//! Conflict-aware retained-delete restore helpers.
//!
//! Explicit restore from Recovery must keep user-visible data safe even when new files now
//! exist at the original paths. This module merges retained staging data back into the source
//! tree file-by-file, reuses exact matches, and preserves both copies when contents differ.

use super::DeleteStagingInfo;
use super::journal::remove_entry;
use std::path::{Path, PathBuf};

#[path = "restore_merge/ops.rs"]
mod ops;
#[path = "restore_merge/util.rs"]
mod util;

/// Outcome recorded for one staged file restored from retained delete recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestoredFileDisposition {
    /// The staged file became the canonical file at its original relative path.
    RestoredCanonical,
    /// The canonical file already matched exactly, so the staged copy was discarded.
    ReusedExisting,
    /// The staged file was preserved under a timestamped conflict path.
    RestoredTimestamped,
}

/// Final path chosen for one staged file during retained restore merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestoredFileRecord {
    /// Original relative path owned by the retained delete entry.
    pub(crate) original_relative: PathBuf,
    /// Final relative path that now contains the staged file's contents.
    pub(crate) final_relative: PathBuf,
    /// How the staged file was resolved.
    pub(crate) disposition: RestoredFileDisposition,
}

/// Timestamped relocation recorded for one pre-existing canonical file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistingFileRelocation {
    /// Canonical relative path that the newer staged file reclaimed.
    pub(crate) original_relative: PathBuf,
    /// Timestamped relative path that now holds the displaced existing file.
    pub(crate) relocated_relative: PathBuf,
}

/// Report describing one explicit retained-delete restore merge.
#[derive(Debug, Default, Clone)]
pub(crate) struct RetainedRestoreMergeReport {
    /// Per-file outcomes for the staged content.
    pub(crate) restored_files: Vec<RestoredFileRecord>,
    /// Existing canonical files that were moved aside before staged content was restored.
    pub(crate) existing_relocations: Vec<ExistingFileRelocation>,
    /// Whether any non-trivial collision handling happened during the merge.
    pub(crate) had_conflicts: bool,
}

impl RetainedRestoreMergeReport {
    /// Look up the recorded staged-file outcome for one original relative path.
    pub(crate) fn restored_record_for(
        &self,
        original_relative: &Path,
    ) -> Option<&RestoredFileRecord> {
        self.restored_files
            .iter()
            .find(|record| record.original_relative == original_relative)
    }
}

/// Restore one retained delete back into the source tree using safe merge semantics.
pub(crate) fn restore_retained_folder_with_merge(
    info: &DeleteStagingInfo,
    source_root: &Path,
    absolute: &Path,
    staging_root: &Path,
) -> Result<RetainedRestoreMergeReport, String> {
    let stamp = ops::utc_conflict_stamp()?;
    restore_retained_folder_with_merge_at(info, source_root, absolute, staging_root, &stamp)
}

fn restore_retained_folder_with_merge_at(
    info: &DeleteStagingInfo,
    source_root: &Path,
    absolute: &Path,
    staging_root: &Path,
    stamp: &str,
) -> Result<RetainedRestoreMergeReport, String> {
    if !info.staged_absolute.is_dir() {
        return Err(format!(
            "Retained staged folder missing: {}",
            info.original_relative.display()
        ));
    }
    let mut report = RetainedRestoreMergeReport::default();
    ops::merge_directory(
        &info.staged_absolute,
        absolute,
        source_root,
        &info.original_relative,
        stamp,
        &mut report,
    )?;
    remove_entry(staging_root, &info.id)?;
    super::cleanup_staging_root(staging_root);
    Ok(report)
}

#[cfg(test)]
mod tests;
