use super::{FolderBrowserMessage, FolderBrowserState, path_id};
use crate::native_app::sample_library::folder_browser::commands::FileMoveConflictResolution;
use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanDiscoveryBatch, scan_source_with_progress,
};
use crate::native_app::sample_library::folder_browser::test_support::{
    FolderDragPreview, MIN_FILE_COLUMN_WIDTH,
};
use radiant::{
    layout::Point,
    runtime::ExternalDragPayload,
    widgets::{DragHandleMessage, PointerModifiers, TextInputMessage},
};
use std::{fs, path::PathBuf};
use wavecrate::sample_sources::SampleCollection;

mod delete;
mod drag_drop;
mod navigation;
mod rename;
mod scanning;
mod source_management;

fn temp_source_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{name}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}
