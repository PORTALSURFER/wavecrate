use std::collections::HashSet;

use crate::native_app::sample_library::folder_browser::{FolderBrowserState, FolderScanProgress};
use crate::native_app::sample_library::source_watcher::GuiSourceWatcherHandle;

pub(in crate::native_app) struct LibraryAppState {
    pub(in crate::native_app) folder_browser: FolderBrowserState,
    pub(in crate::native_app) folder_progress: Option<FolderScanProgress>,
    pub(in crate::native_app) pending_source_refreshes: HashSet<String>,
    pub(in crate::native_app) source_watcher: Option<GuiSourceWatcherHandle>,
}

impl LibraryAppState {
    pub(in crate::native_app) fn new(
        folder_browser: FolderBrowserState,
        source_watcher: Option<GuiSourceWatcherHandle>,
    ) -> Self {
        Self {
            folder_browser,
            folder_progress: None,
            pending_source_refreshes: HashSet::new(),
            source_watcher,
        }
    }
}
