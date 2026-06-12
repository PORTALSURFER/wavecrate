//! Aggregate UI cache composition for controller code that still needs one root.

use super::{BrowserCacheState, FolderBrowsersState};

pub(crate) struct ControllerUiCacheState {
    pub(crate) browser: BrowserCacheState,
    pub(crate) folders: FolderBrowsersState,
}

impl ControllerUiCacheState {
    pub(crate) fn new() -> Self {
        Self {
            browser: BrowserCacheState::new(),
            folders: FolderBrowsersState::new(),
        }
    }
}
