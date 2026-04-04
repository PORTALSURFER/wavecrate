use super::SampleBrowserState;
use crate::sample_sources::SourceId;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Focus, multi-selection, and marker-cache state for the sample browser.
#[derive(Clone, Debug, Default)]
pub struct BrowserSelectionState {
    /// Focused row used for playback/navigation (mirrors previously “selected”).
    pub selected: Option<super::SampleBrowserIndex>,
    /// Loaded row used for playback.
    pub loaded: Option<super::SampleBrowserIndex>,
    /// Visible row indices for selection/autoscroll (filtered list).
    pub selected_visible: Option<usize>,
    /// Visible index for the loaded row, if any.
    pub loaded_visible: Option<usize>,
    /// Visible row anchor used for range selection (shift + click/arrow).
    pub selection_anchor_visible: Option<usize>,
    /// Paths currently included in the browser multi-selection set.
    pub selected_paths: Vec<PathBuf>,
    /// Cached absolute indices derived from `selected_paths` for index-driven callers.
    pub selected_indices_cache: BrowserSelectedIndicesCache,
    /// Monotonic revision bumped whenever the browser multi-selection changes.
    pub selected_paths_revision: u64,
    /// Last marker-input snapshot used to short-circuit redundant marker recomputes.
    pub marker_cache: Option<BrowserMarkerCacheState>,
    /// Last focused browser item to restore focus after context changes.
    pub last_focused_index: Option<usize>,
    /// Last focused browser item to restore focus after context changes.
    pub last_focused_path: Option<PathBuf>,
    /// Whether the current browser focus was only previewed and still needs commit-time effects.
    pub commit_focus_pending: bool,
    /// Whether autoscroll is enabled for selection changes.
    pub autoscroll: bool,
}

/// Snapshot of marker-driving browser inputs for redundant-refresh short-circuiting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrowserMarkerCacheState {
    /// Visible-row projection revision used by focus/loaded marker lookup.
    pub visible_rows_revision: u64,
    /// Stable hash of the focused wav path when present.
    pub selected_path_hash: Option<u64>,
    /// Stable hash of the loaded wav path when present.
    pub loaded_path_hash: Option<u64>,
    /// Stable hash of the browser multi-selection identity set.
    pub selected_paths_hash: u64,
    /// Current range-selection anchor in visible-row coordinates.
    pub selection_anchor_visible: Option<usize>,
}

impl BrowserMarkerCacheState {
    /// Build a marker cache snapshot from browser state and current focused/loaded paths.
    pub fn from_inputs(
        browser: &SampleBrowserState,
        selected_path: Option<&std::path::Path>,
        loaded_path: Option<&std::path::Path>,
    ) -> Self {
        Self {
            visible_rows_revision: browser.viewport.visible_rows_revision,
            selected_path_hash: selected_path.map(hash_path),
            loaded_path_hash: loaded_path.map(hash_path),
            selected_paths_hash: hash_paths(&browser.selection.selected_paths),
            selection_anchor_visible: browser.selection.selection_anchor_visible,
        }
    }
}

fn hash_path(path: &std::path::Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

fn hash_paths(paths: &[PathBuf]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    paths.hash(&mut hasher);
    hasher.finish()
}

/// Cached absolute-index projection of the authoritative browser selection paths.
#[derive(Clone, Debug, Default)]
pub struct BrowserSelectedIndicesCache {
    /// Selection revision for which `indices` is valid.
    pub revision: u64,
    /// Source id for which `indices` is valid.
    pub source_id: Option<SourceId>,
    /// Source DB revision for which `indices` is valid.
    pub source_revision: Option<u64>,
    /// Total wav-entry count for which `indices` is valid.
    pub entries_len: usize,
    /// Absolute entry indices derived from `SampleBrowserState::selected_paths`.
    pub indices: Vec<usize>,
}
