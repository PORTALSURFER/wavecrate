use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderDeleteTargetView {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileDeleteTargetView {
    pub(in crate::native_app) paths: Vec<PathBuf>,
    pub(in crate::native_app) names: Vec<String>,
}
