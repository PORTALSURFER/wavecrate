use super::commands::FolderBrowserMessage;
use super::{FolderBrowserState, path_id};
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

#[test]
fn root_facade_exports_only_stable_entrypoints() {
    let source = include_str!("../../folder_browser.rs");
    let root_exports = source
        .lines()
        .filter(|line| line.starts_with("pub(in crate::native_app) use "))
        .collect::<Vec<_>>();

    assert_eq!(
        root_exports,
        vec!["pub(in crate::native_app) use state::FolderBrowserState;"]
    );
    for module in [
        "commands",
        "model",
        "projection",
        "scan",
        "test_support",
        "view_contract",
    ] {
        assert!(
            source.contains(&format!("pub(in crate::native_app) mod {module}")),
            "folder-browser API group `{module}` should stay explicit"
        );
    }
}
