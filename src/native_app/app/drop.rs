use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct NativeFileDropHover {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) supported: bool,
}
