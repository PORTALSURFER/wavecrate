use super::FolderBrowserUiState;
use crate::sample_sources::SourceId;

/// Stable identifier for one of the two sidebar folder panes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FolderPaneId {
    /// Upper folder pane shown directly beneath the shared sources list.
    #[default]
    Upper,
    /// Lower folder pane shown beneath the upper pane.
    Lower,
}

impl FolderPaneId {
    /// Return the opposite pane for simple upper/lower toggles.
    pub fn other(self) -> Self {
        match self {
            Self::Upper => Self::Lower,
            Self::Lower => Self::Upper,
        }
    }

    /// Return a small stable string used by config persistence and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }
}

/// Folder-pane assignment plus retained tree/search state for one pane.
#[derive(Clone, Debug, Default)]
pub struct FolderPaneState {
    /// Source currently shown in this pane, if any.
    pub source_id: Option<SourceId>,
    /// Whether this pane is hydrating its assigned source snapshot.
    pub loading: bool,
    /// Whether this pane is asynchronously rebuilding its folder-tree rows.
    pub projecting: bool,
    /// Retained browser state for this pane when it is not active.
    pub browser: FolderBrowserUiState,
}

/// Fixed set of the two folder panes shown in the sidebar.
#[derive(Clone, Debug, Default)]
pub struct FolderPaneStateSet {
    /// Upper folder pane state.
    pub upper: FolderPaneState,
    /// Lower folder pane state.
    pub lower: FolderPaneState,
}

impl FolderPaneStateSet {
    /// Borrow one pane state by id.
    pub fn get(&self, pane: FolderPaneId) -> &FolderPaneState {
        match pane {
            FolderPaneId::Upper => &self.upper,
            FolderPaneId::Lower => &self.lower,
        }
    }

    /// Mutably borrow one pane state by id.
    pub fn get_mut(&mut self, pane: FolderPaneId) -> &mut FolderPaneState {
        match pane {
            FolderPaneId::Upper => &mut self.upper,
            FolderPaneId::Lower => &mut self.lower,
        }
    }
}
