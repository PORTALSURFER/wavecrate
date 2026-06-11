use std::path::PathBuf;

use radiant::gui::types::Point;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct FolderDropResult {
    pub(in crate::native_app) moved_paths: Vec<(PathBuf, PathBuf)>,
    pub(in crate::native_app) status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct FolderDragPreview {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) pointer: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct FileColumnDragFeedback {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) pointer: Point,
    pub(in crate::native_app) width: f32,
    pub(in crate::native_app) marker_x: f32,
}
