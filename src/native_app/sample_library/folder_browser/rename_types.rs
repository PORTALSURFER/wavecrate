use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenamePathRemap {
    pub(in crate::native_app) old_path: PathBuf,
    pub(in crate::native_app) new_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenameCommitResult {
    pub(in crate::native_app) status: String,
    pub(in crate::native_app) path_remap: Option<RenamePathRemap>,
}

impl RenameCommitResult {
    pub(in crate::native_app) fn status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            path_remap: None,
        }
    }

    pub(in crate::native_app) fn remapped(
        status: impl Into<String>,
        old_path: PathBuf,
        new_path: PathBuf,
    ) -> Self {
        Self {
            status: status.into(),
            path_remap: Some(RenamePathRemap { old_path, new_path }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileRenameView {
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) input_id: u64,
    pub(in crate::native_app) selection_start: usize,
    pub(in crate::native_app) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenameTargetView {
    pub(in crate::native_app) kind: &'static str,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) is_source_root: bool,
}
