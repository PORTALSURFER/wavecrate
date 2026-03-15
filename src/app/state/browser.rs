mod search;
mod selection;
mod viewport;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub use search::*;
pub use selection::*;
pub use viewport::*;

/// Sample browser state for wav entries with filterable rows.
#[derive(Clone, Debug)]
pub struct SampleBrowserState {
    /// Absolute indices per tag for keyboard navigation and tagging.
    pub trash: Arc<[usize]>,
    /// Absolute indices for neutral-tagged rows.
    pub neutral: Arc<[usize]>,
    /// Absolute indices for keep-tagged rows.
    pub keep: Arc<[usize]>,
    /// Focus, multi-selection, and marker-cache state.
    pub selection: BrowserSelectionState,
    /// Visible-row projection and reverse-lookup caches.
    pub viewport: BrowserViewportState,
    /// Search/filter/similarity state for the browser list.
    pub search: BrowserSearchState,
    /// Pending inline action for the sample browser rows.
    pub pending_action: Option<SampleBrowserActionPrompt>,
    /// Flag to request focus on the active inline rename editor.
    pub rename_focus_requested: bool,
    /// Active tab in the sample browser area.
    pub active_tab: SampleBrowserTab,
    /// Paths that should flash in the browser list after a copy action.
    pub copy_flash_paths: Vec<PathBuf>,
    /// Start time for the current browser copy flash.
    pub copy_flash_at: Option<Instant>,
}

impl Default for SampleBrowserState {
    fn default() -> Self {
        Self {
            trash: Arc::from([]),
            neutral: Arc::from([]),
            keep: Arc::from([]),
            selection: BrowserSelectionState::default(),
            viewport: BrowserViewportState::default(),
            search: BrowserSearchState::default(),
            pending_action: None,
            rename_focus_requested: false,
            active_tab: SampleBrowserTab::List,
            copy_flash_paths: Vec::new(),
            copy_flash_at: None,
        }
    }
}

/// Pending inline action for the sample browser.
#[derive(Clone, Debug)]
pub enum SampleBrowserActionPrompt {
    /// Rename the selected entry.
    Rename {
        /// Path to rename.
        target: PathBuf,
        /// New name.
        name: String,
    },
}

/// Tabs for the sample browser area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserTab {
    /// List view tab.
    List,
    /// Map view tab.
    Map,
}
