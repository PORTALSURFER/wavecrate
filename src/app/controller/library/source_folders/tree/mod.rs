//! Folder-browser orchestration across retained state, scan refresh, and row projection.

use super::*;
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use crate::app::state::FolderPaneId;
use std::time::Duration;

const SHOW_ALL_FOLDERS_SCAN_MAX_AGE: Duration = Duration::from_secs(10);

mod model;
mod projection;
mod scan;

pub(crate) use model::FolderBrowserModel;
pub(crate) use scan::scan_disk_folders;

impl AppController {
    pub(crate) fn refresh_folder_browser(&mut self) {
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            self.ui.sources.folders = FolderBrowserUiState::default();
            self.sync_active_folder_ui_to_pane();
            return;
        };
        let Some(source) = self.current_source() else {
            self.ui.sources.folders = FolderBrowserUiState::default();
            self.sync_active_folder_ui_to_pane();
            return;
        };
        let pane = self.active_folder_pane();
        let cache_key = folder_browser_cache_key(pane, source_id.clone());
        let pending_load = self.runtime.jobs.wav_load_pending_for(&source.id);
        let empty_entries = self.wav_entries_len() == 0;
        let (cached_available, cached_available_show_all_folders, cached_disk) = {
            let model = self
                .ui_cache
                .folders
                .models
                .entry(cache_key.clone())
                .or_default();
            (
                model.available.clone(),
                model.available_show_all_folders,
                model.disk_folders.clone(),
            )
        };
        self.request_folder_browser_disk_scan_if_needed(
            &source_id,
            &source.root,
            SHOW_ALL_FOLDERS_SCAN_MAX_AGE,
        );
        let show_all_folders = self
            .ui_cache
            .folders
            .models
            .get(&cache_key)
            .map(|model| model.show_all_folders)
            .unwrap_or(true);
        let mut available = self.collect_folders(&source.root, false);
        if show_all_folders {
            available.extend(cached_disk);
        }
        let reuse_available = empty_entries
            && !cached_available.is_empty()
            && available.is_empty()
            && cached_available_show_all_folders == show_all_folders;
        if reuse_available
            || (pending_load
                && empty_entries
                && available.is_empty()
                && cached_available_show_all_folders == show_all_folders)
        {
            available = cached_available;
        }
        let snapshot = {
            let model = self
                .ui_cache
                .folders
                .models
                .entry(cache_key)
                .or_default();
            model
                .manual_folders
                .retain(|path| source.root.join(path).is_dir());
            model
                .hotkeys
                .retain(|_, path| model::is_root_path(path) || source.root.join(path).is_dir());
            model.available = available;
            model.available_show_all_folders = model.show_all_folders;
            for path in model.manual_folders.iter().cloned() {
                model.available.insert(path);
            }
            model
                .selected
                .retain(|path| model::is_root_path(path) || model.available.contains(path));
            model
                .negated
                .retain(|path| model::is_root_path(path) || model.available.contains(path));
            model.expanded.retain(|path| model.available.contains(path));
            if model.expanded.is_empty() {
                for dir in model.available.iter().filter(|path| {
                    path.parent()
                        .is_none_or(|parent| parent.as_os_str().is_empty())
                }) {
                    model.expanded.insert(dir.clone());
                }
            }
            model.clear_focus_if_missing();
            model.clear_anchor_if_missing();
            for path in model.selected.iter() {
                let mut cursor = path.as_path();
                while let Some(parent) = cursor.parent() {
                    model.expanded.insert(parent.to_path_buf());
                    cursor = parent;
                }
            }
            model.clone()
        };
        self.set_ui_folder_search_query(snapshot.search_query.clone());
        self.build_folder_rows(&snapshot);
        self.sync_active_folder_ui_to_pane();
    }

    pub(crate) fn current_folder_model_mut(&mut self) -> Option<&mut FolderBrowserModel> {
        let id = self.selection_state.ctx.selected_source.clone()?;
        let key = folder_browser_cache_key(self.active_folder_pane(), id);
        Some(self.ui_cache.folders.models.entry(key).or_default())
    }

    pub(crate) fn current_folder_model(&self) -> Option<&FolderBrowserModel> {
        let id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache
            .folders
            .models
            .get(&folder_browser_cache_key(self.active_folder_pane(), id.clone()))
    }
}

fn folder_browser_cache_key(pane: FolderPaneId, source_id: SourceId) -> FolderBrowserCacheKey {
    FolderBrowserCacheKey { pane, source_id }
}
