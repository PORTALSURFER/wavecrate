use super::{
    DropTargetsUiState, FolderBrowserUiState, FolderPaneId, FolderPaneState, FolderPaneStateSet,
};
use crate::sample_sources::SourceId;

/// Sidebar list of sample sources.
#[derive(Clone, Debug, Default)]
pub struct SourcePanelState {
    /// Render rows for configured sources.
    pub rows: Vec<SourceRowView>,
    /// Currently selected row index.
    pub selected: Option<usize>,
    /// Source row currently hydrating in the background, when any.
    pub loading_source_id: Option<SourceId>,
    /// Row index with an open context menu.
    pub menu_row: Option<usize>,
    /// Row index to scroll into view.
    pub scroll_to: Option<usize>,
    /// User-defined height for the sources list section, excluding its header.
    pub sources_height_override: Option<f32>,
    /// Cached list height at the start of a sources resize drag for stable deltas.
    pub sources_resize_origin_height: Option<f32>,
    /// Active folder browser sub-state used by current controller projections.
    pub folders: FolderBrowserUiState,
    /// Retained folder-pane assignments and inactive-pane UI state.
    pub folder_panes: FolderPaneStateSet,
    /// Folder pane that currently drives the sample browser and waveform.
    pub active_folder_pane: FolderPaneId,
    /// Drop target sub-state.
    pub drop_targets: DropTargetsUiState,
}

impl SourcePanelState {
    /// Borrow one pane assignment by id.
    pub fn folder_pane(&self, pane: FolderPaneId) -> &FolderPaneState {
        self.folder_panes.get(pane)
    }

    /// Mutably borrow one pane assignment by id.
    pub fn folder_pane_mut(&mut self, pane: FolderPaneId) -> &mut FolderPaneState {
        self.folder_panes.get_mut(pane)
    }
}

/// Display data for a single source row.
#[derive(Clone, Debug)]
pub struct SourceRowView {
    /// Source identifier.
    pub id: SourceId,
    /// Display name.
    pub name: String,
    /// Display path.
    pub path: String,
    /// Whether the source is missing on disk.
    pub missing: bool,
}
