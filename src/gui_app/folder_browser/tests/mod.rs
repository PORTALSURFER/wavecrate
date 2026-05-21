use super::{
    FolderBrowserMessage, FolderBrowserState, FolderDragPreview, FolderScanDiscoveryBatch,
    MIN_FILE_COLUMN_WIDTH, path_id, scan_source_with_progress,
};
use radiant::{
    layout::Point,
    runtime::ExternalDragPayload,
    widgets::{DragHandleMessage, PointerModifiers, TextInputMessage},
};
use std::{fs, path::PathBuf};

mod delete;
mod drag_drop;
mod navigation;
mod rename;
mod scanning;

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
