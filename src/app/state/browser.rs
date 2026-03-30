mod duplicate_cleanup;
mod marks;
mod search;
mod selection;
mod viewport;

use crate::sample_sources::SourceId;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub use duplicate_cleanup::*;
pub use marks::*;
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
    /// Session-scoped temporary sample marks keyed by source-relative path.
    pub marks: BrowserMarkedState,
    /// Active duplicate-cleanup workspace for the browser list.
    pub duplicate_cleanup: Option<BrowserDuplicateCleanupState>,
    /// Pending modal action for the sample browser area.
    pub pending_action: Option<SampleBrowserActionPrompt>,
    /// Flag to request focus on the active prompt input field.
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
            marks: BrowserMarkedState::default(),
            duplicate_cleanup: None,
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
        /// Inline validation error shown under the prompt input when present.
        input_error: Option<String>,
    },
    /// Complete a blocked folder drop by choosing a unique destination name.
    MoveToFolderConflict {
        /// Source identifier for the dragged sample.
        source_id: SourceId,
        /// Path of the source sample within the source root.
        source_relative: PathBuf,
        /// Destination folder that rejected the original drop.
        target_folder: PathBuf,
        /// Proposed destination name without forcing the extension into the input text.
        name: String,
        /// Inline validation error shown under the prompt input when present.
        input_error: Option<String>,
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
