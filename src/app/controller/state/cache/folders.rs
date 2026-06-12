//! Folder-browser projection caches keyed by pane and source.

use crate::app::controller::library::source_folders;
use crate::app::state::FolderPaneId;
use crate::sample_sources::SourceId;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FolderBrowserCacheKey {
    pub(crate) pane: FolderPaneId,
    pub(crate) source_id: SourceId,
}

pub(crate) struct FolderBrowsersState {
    pub(crate) models: HashMap<FolderBrowserCacheKey, source_folders::FolderBrowserModel>,
    pub(crate) snapshots: HashMap<FolderBrowserCacheKey, source_folders::FolderTreeSnapshot>,
}

impl FolderBrowsersState {
    pub(crate) fn new() -> Self {
        Self {
            models: HashMap::new(),
            snapshots: HashMap::new(),
        }
    }
}
