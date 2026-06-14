use super::*;
use crate::app::state::{FolderFileScopeMode, FolderPaneId};
use std::collections::{BTreeMap, BTreeSet};

/// Reversible source, pane, and folder-browser state.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SourceFolderHistorySnapshot {
    selected_source: Option<SourceId>,
    upper_folder_pane_source: Option<SourceId>,
    lower_folder_pane_source: Option<SourceId>,
    active_folder_pane: FolderPaneId,
    last_selected_browsable_source: Option<SourceId>,
    folder_ui_focused: Option<usize>,
    folder_ui_last_focused_path: Option<PathBuf>,
    folder_state: Option<FolderHistorySnapshot>,
}

/// Reversible folder-browser state owned by one selected source.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FolderHistorySnapshot {
    selected: BTreeSet<PathBuf>,
    negated: BTreeSet<PathBuf>,
    expanded: BTreeSet<PathBuf>,
    focused: Option<PathBuf>,
    selection_anchor: Option<PathBuf>,
    manual_folders: BTreeSet<PathBuf>,
    hotkeys: BTreeMap<u8, PathBuf>,
    show_all_folders: bool,
    file_scope_mode: FolderFileScopeMode,
}

pub(super) fn capture_source_folder_snapshot(
    controller: &AppController,
) -> SourceFolderHistorySnapshot {
    SourceFolderHistorySnapshot {
        selected_source: controller.selection_state.ctx.selected_source.clone(),
        upper_folder_pane_source: controller.folder_pane_source(FolderPaneId::Upper),
        lower_folder_pane_source: controller.folder_pane_source(FolderPaneId::Lower),
        active_folder_pane: controller.ui.sources.active_folder_pane,
        last_selected_browsable_source: controller
            .selection_state
            .ctx
            .last_selected_browsable_source
            .clone(),
        folder_ui_focused: controller.ui.sources.folders.focused,
        folder_ui_last_focused_path: controller.ui.sources.folders.last_focused_path.clone(),
        folder_state: capture_folder_state(controller),
    }
}

pub(super) fn restore_source_folder_snapshot(
    controller: &mut AppController,
    snapshot: &SourceFolderHistorySnapshot,
) {
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = snapshot.last_selected_browsable_source.clone();
    controller.sync_active_folder_ui_to_pane();
    controller.ui.sources.folder_panes.upper.source_id = snapshot.upper_folder_pane_source.clone();
    controller.ui.sources.folder_panes.lower.source_id = snapshot.lower_folder_pane_source.clone();
    controller.ui.sources.active_folder_pane = snapshot.active_folder_pane;
    controller.selection_state.ctx.selected_source = snapshot.selected_source.clone();
    controller.load_active_folder_ui_from_pane();
    restore_folder_state(controller, snapshot);
    controller.refresh_sources_ui();
    controller.refresh_folder_browser();
    controller.ui.sources.folders.focused = snapshot.folder_ui_focused;
    controller.ui.sources.folders.scroll_to = snapshot.folder_ui_focused;
    controller.ui.sources.folders.last_focused_path = snapshot.folder_ui_last_focused_path.clone();
}

fn capture_folder_state(controller: &AppController) -> Option<FolderHistorySnapshot> {
    let folder_cache_key = controller
        .selection_state
        .ctx
        .selected_source
        .as_ref()
        .map(
            |source_id| crate::app::controller::state::cache::FolderBrowserCacheKey {
                pane: controller.ui.sources.active_folder_pane,
                source_id: source_id.clone(),
            },
        )?;
    controller
        .ui_cache
        .folders
        .models
        .get(&folder_cache_key)
        .map(|model| FolderHistorySnapshot {
            selected: model.selected.clone(),
            negated: model.negated.clone(),
            expanded: model.expanded.clone(),
            focused: model.focused.clone(),
            selection_anchor: model.selection_anchor.clone(),
            manual_folders: model.manual_folders.clone(),
            hotkeys: model.hotkeys.clone(),
            show_all_folders: model.show_all_folders,
            file_scope_mode: model.file_scope_mode,
        })
}

fn restore_folder_state(controller: &mut AppController, snapshot: &SourceFolderHistorySnapshot) {
    let Some(source_id) = snapshot.selected_source.clone() else {
        return;
    };
    let model = controller
        .ui_cache
        .folders
        .models
        .entry(
            crate::app::controller::state::cache::FolderBrowserCacheKey {
                pane: controller.ui.sources.active_folder_pane,
                source_id,
            },
        )
        .or_default();
    if let Some(folder_state) = snapshot.folder_state.as_ref() {
        model.selected = folder_state.selected.clone();
        model.negated = folder_state.negated.clone();
        model.expanded = folder_state.expanded.clone();
        model.focused = folder_state.focused.clone();
        model.selection_anchor = folder_state.selection_anchor.clone();
        model.manual_folders = folder_state.manual_folders.clone();
        model.hotkeys = folder_state.hotkeys.clone();
        model.show_all_folders = folder_state.show_all_folders;
        model.file_scope_mode = folder_state.file_scope_mode;
    }
}
