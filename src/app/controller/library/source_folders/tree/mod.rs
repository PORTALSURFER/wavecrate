//! Folder-browser orchestration across retained state, scan refresh, and row projection.

use super::*;
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use crate::app::state::FolderPaneId;
use std::time::Duration;

const SHOW_ALL_FOLDERS_SCAN_MAX_AGE: Duration = Duration::from_secs(10);

mod async_projection;
mod model;
mod projection;
mod scan;
mod snapshot;

#[cfg(test)]
pub(crate) use async_projection::with_folder_projection_async_enabled_for_tests;
pub(crate) use model::FolderBrowserModel;
pub(crate) use projection::FolderProjectionView;
pub(crate) use scan::scan_disk_folders;
pub(crate) use snapshot::FolderTreeSnapshot;

impl AppController {
    pub(crate) fn refresh_folder_browser(&mut self) {
        self.queue_folder_browser_refresh();
    }

    pub(crate) fn current_folder_model_mut(&mut self) -> Option<&mut FolderBrowserModel> {
        let id = self.selection_state.ctx.selected_source.clone()?;
        let key = folder_browser_cache_key(self.active_folder_pane(), id);
        Some(self.ui_cache.folders.models.entry(key).or_default())
    }

    pub(crate) fn current_folder_model(&self) -> Option<&FolderBrowserModel> {
        let id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache.folders.models.get(&folder_browser_cache_key(
            self.active_folder_pane(),
            id.clone(),
        ))
    }
}

fn folder_browser_cache_key(pane: FolderPaneId, source_id: SourceId) -> FolderBrowserCacheKey {
    FolderBrowserCacheKey { pane, source_id }
}
