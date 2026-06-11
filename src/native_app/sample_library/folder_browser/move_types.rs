use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMoveConflictResolution {
    Overwrite,
    Rename,
    Skip,
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
