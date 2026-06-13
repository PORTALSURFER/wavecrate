use super::super::jobs::{FolderProjectionResult, FolderProjectionSnapshot, JobMessage};
use super::super::library::source_folders::with_folder_projection_async_enabled_for_tests;
use super::super::library::wavs::with_browser_async_pipeline_enabled_for_tests;
use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use crate::app::controller::library::source_folders::{FolderProjectionView, FolderTreeSnapshot};
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod browser_dispatch;
mod focus_selection;
mod pane_cancellation;
mod projection_queue;
mod search_projection;

fn folder_row_index(controller: &AppController, path: &str) -> usize {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from(path))
        .unwrap_or_else(|| panic!("missing folder row for {path}"))
}

fn visible_folder_paths(controller: &AppController) -> Vec<PathBuf> {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect()
}

fn nested_folder_controller() -> (AppController, SampleSource) {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.assign_source_to_folder_pane(
        crate::app::state::FolderPaneId::Upper,
        Some(source.id.clone()),
    );
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir_all(source.root.join("extra-empty")).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller
        .set_wav_entries_for_tests(vec![sample_entry("drums/kicks/tight.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    (controller, source)
}

fn pending_projection_request_id(controller: &AppController) -> u64 {
    controller
        .runtime
        .source_lane
        .folder_projection
        .pending_for_tests(crate::app::state::FolderPaneId::Upper)
        .expect("pending folder projection")
        .request_id
}

fn projection_result(
    controller: &AppController,
    source: &SampleSource,
    request_id: u64,
    rows: Vec<crate::app::state::FolderRowView>,
    focused: Option<usize>,
) -> FolderProjectionResult {
    let pane = crate::app::state::FolderPaneId::Upper;
    let key = crate::app::controller::state::cache::FolderBrowserCacheKey {
        pane,
        source_id: source.id.clone(),
    };
    let model = controller
        .ui_cache
        .folders
        .models
        .get(&key)
        .cloned()
        .expect("folder model");
    let tree = controller
        .ui_cache
        .folders
        .snapshots
        .get(&key)
        .cloned()
        .unwrap_or_else(|| FolderTreeSnapshot::from_available(&model.available));
    let flattened_view =
        model.file_scope_mode == crate::app::state::FolderFileScopeMode::AllDescendants;
    let search_query = model.search_query.clone();
    let show_all_folders = model.show_all_folders;
    FolderProjectionResult {
        request_id,
        pane,
        source_id: source.id.clone(),
        elapsed: Duration::from_millis(5),
        snapshot: FolderProjectionSnapshot {
            model,
            tree,
            view: FolderProjectionView {
                rows,
                focused,
                scroll_to: focused,
                search_query,
                show_all_folders,
                flattened_view,
            },
        },
    }
}

fn root_row(has_children: bool) -> crate::app::state::FolderRowView {
    crate::app::state::FolderRowView {
        path: PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        file_scope_mode: Some(crate::app::state::FolderFileScopeMode::DirectOnly),
    }
}

fn row(
    path: &str,
    depth: usize,
    has_children: bool,
    expanded: bool,
) -> crate::app::state::FolderRowView {
    let path_buf = PathBuf::from(path);
    crate::app::state::FolderRowView {
        name: path_buf
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string()),
        path: path_buf,
        depth,
        has_children,
        expanded,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        file_scope_mode: None,
    }
}
