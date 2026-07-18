use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RemovedSource {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) was_selected: bool,
}
