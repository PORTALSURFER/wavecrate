use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMoveConflictResolution {
    Overwrite,
    Rename,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflictResolutionRequest {
    pub(in crate::native_app) resolution: FileMoveConflictResolution,
    pub(in crate::native_app) apply_to_remaining: bool,
}

impl FileMoveConflictResolutionRequest {
    pub(in crate::native_app) fn new(
        resolution: FileMoveConflictResolution,
        apply_to_remaining: bool,
    ) -> Self {
        Self {
            resolution,
            apply_to_remaining,
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn apply_to_remaining(
        resolution: FileMoveConflictResolution,
    ) -> Self {
        Self::new(resolution, true)
    }
}

impl From<FileMoveConflictResolution> for FileMoveConflictResolutionRequest {
    fn from(resolution: FileMoveConflictResolution) -> Self {
        Self::new(resolution, false)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflict {
    pub(in crate::native_app) source_path: PathBuf,
    pub(in crate::native_app) destination_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflictBatch {
    pub(in crate::native_app) target_folder: PathBuf,
    pub(in crate::native_app) conflicts: Vec<FileMoveConflict>,
    pub(in crate::native_app) current_index: usize,
    pub(in crate::native_app) resolved_count: usize,
    pub(in crate::native_app) skipped_count: usize,
    pub(in crate::native_app) batch_policy: Option<FileMoveConflictResolution>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflictView {
    pub(in crate::native_app) source_path: PathBuf,
    pub(in crate::native_app) destination_path: PathBuf,
    pub(in crate::native_app) file_name: String,
    pub(in crate::native_app) destination_folder: String,
    pub(in crate::native_app) current_number: usize,
    pub(in crate::native_app) total_count: usize,
}
