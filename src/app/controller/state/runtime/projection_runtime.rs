//! Runtime state for projection revisions and derived invalidation.

use super::derived_graph::DerivedStateGraph;

/// Bitmask of pending projection revision bumps set at mutation time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectionRevisionDirtyMask(pub(crate) u16);

impl ProjectionRevisionDirtyMask {
    /// No pending revision work.
    pub(crate) const NONE: u16 = 0;
    /// Status text/tone revision is dirty.
    pub(crate) const STATUS: u16 = 1 << 0;
    /// Folder-search query revision is dirty.
    pub(crate) const FOLDER_SEARCH: u16 = 1 << 1;
    /// Browser-search query revision is dirty.
    pub(crate) const BROWSER_SEARCH: u16 = 1 << 2;
    /// Browser-row inline metadata revision is dirty.
    pub(crate) const BROWSER_ROW_METADATA: u16 = 1 << 3;
    /// Map selection revision is dirty.
    pub(crate) const MAP_SELECTION: u16 = 1 << 4;
    /// Map hover revision is dirty.
    pub(crate) const MAP_HOVER: u16 = 1 << 5;
    /// Map dataset identity revision is dirty.
    pub(crate) const MAP_DATASET: u16 = 1 << 6;
    /// Map query-bounds revision is dirty.
    pub(crate) const MAP_QUERY: u16 = 1 << 7;
    /// Update panel revision is dirty.
    pub(crate) const UPDATE: u16 = 1 << 8;
    /// Loaded wav path revision is dirty.
    pub(crate) const LOADED_WAV: u16 = 1 << 9;
}

/// Projection invalidation runtime state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProjectionRuntimeState {
    /// Incremental derived-state dirty graph used by UI projection paths.
    pub(crate) derived_graph: DerivedStateGraph,
    /// Pending projection revision bumps recorded by mutation paths.
    pub(crate) revision_dirty: ProjectionRevisionDirtyMask,
}
