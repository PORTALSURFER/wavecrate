use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Sample browser state for wav entries with filterable rows.
#[derive(Clone, Debug)]
pub struct SampleBrowserState {
    /// Absolute indices per tag for keyboard navigation and tagging.
    pub trash: Arc<[usize]>,
    /// Absolute indices for neutral-tagged rows.
    pub neutral: Arc<[usize]>,
    /// Absolute indices for keep-tagged rows.
    pub keep: Arc<[usize]>,
    /// Visible rows after applying the active filter.
    pub visible: VisibleRows,
    /// Monotonic revision bumped whenever the visible-row projection changes.
    pub visible_rows_revision: u64,
    /// Revision for the current absolute-index lookup maps.
    ///
    /// This tracks which visible-row projection revision last rebuilt
    /// `visible_row_by_absolute` and `triage_index_by_absolute`.
    pub lookup_maps_revision: u64,
    /// Focused row used for playback/navigation (mirrors previously “selected”).
    pub selected: Option<SampleBrowserIndex>,
    /// Loaded row used for playback.
    pub loaded: Option<SampleBrowserIndex>,
    /// Visible row indices for selection/autoscroll (filtered list).
    pub selected_visible: Option<usize>,
    /// Visible index for the loaded row, if any.
    pub loaded_visible: Option<usize>,
    /// Visible row anchor used for range selection (shift + click/arrow).
    pub selection_anchor_visible: Option<usize>,
    /// First visible-row index currently projected into the native browser window.
    ///
    /// Native-shell row virtualization uses this to keep the current list window
    /// stable and only scroll when focus approaches the top or bottom edge.
    pub render_window_start: usize,
    /// Cached visible-row lookup by absolute wav-entry index.
    pub visible_row_by_absolute: Vec<Option<usize>>,
    /// Cached triage-column lookup by absolute wav-entry index.
    pub triage_index_by_absolute: Vec<Option<SampleBrowserIndex>>,
    /// Paths currently included in the multi-selection set.
    pub selected_paths: Vec<PathBuf>,
    /// Monotonic revision bumped whenever `selected_paths` changes.
    pub selected_paths_revision: u64,
    /// Last marker-input snapshot used to short-circuit redundant marker recomputes.
    pub marker_cache: Option<BrowserMarkerCacheState>,
    /// Last focused browser item to restore focus after context changes.
    pub last_focused_path: Option<PathBuf>,
    /// Whether autoscroll is enabled for selection changes.
    pub autoscroll: bool,
    /// Active triage filter.
    pub filter: TriageFlagFilter,
    /// Rating levels selected for filtering (-3..=3). Empty means no rating filter.
    pub rating_filter: BTreeSet<i8>,
    /// Text query applied to visible rows via fuzzy search.
    pub search_query: String,
    /// Flag to request focus for the search field in the UI.
    pub search_focus_requested: bool,
    /// When enabled, Up/Down jump through random samples instead of list order.
    pub random_navigation_mode: bool,
    /// Sorting mode for the sample browser list.
    pub sort: SampleBrowserSort,
    /// True when similarity sorting should follow the loaded sample.
    pub similarity_sort_follow_loaded: bool,
    /// Optional similar-sounds filter scoped to the current source.
    pub similar_query: Option<SimilarQuery>,
    /// Near-duplicate highlight set for the focused sample.
    pub focused_similarity: Option<FocusedSimilarity>,
    /// Pending inline action for the sample browser rows.
    pub pending_action: Option<SampleBrowserActionPrompt>,
    /// Flag to request focus on the active inline rename editor.
    pub rename_focus_requested: bool,
    /// Active tab in the sample browser area.
    pub active_tab: SampleBrowserTab,
    /// True when a background search/filter job is running.
    pub search_busy: bool,
    /// Latest issued browser search request identifier.
    pub latest_search_request_id: u64,
    /// Latest browser search request identifier applied to visible rows.
    pub latest_applied_search_request_id: u64,
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
            visible: VisibleRows::List(Vec::new().into()),
            visible_rows_revision: 0,
            lookup_maps_revision: 0,
            selected: None,
            loaded: None,
            selected_visible: None,
            loaded_visible: None,
            selection_anchor_visible: None,
            render_window_start: 0,
            visible_row_by_absolute: Vec::new(),
            triage_index_by_absolute: Vec::new(),
            selected_paths: Vec::new(),
            selected_paths_revision: 0,
            marker_cache: None,
            last_focused_path: None,
            autoscroll: false,
            filter: TriageFlagFilter::All,
            rating_filter: BTreeSet::new(),
            search_query: String::new(),
            search_focus_requested: false,
            random_navigation_mode: false,
            sort: SampleBrowserSort::ListOrder,
            similarity_sort_follow_loaded: false,
            similar_query: None,
            focused_similarity: None,
            pending_action: None,
            rename_focus_requested: false,
            active_tab: SampleBrowserTab::List,
            search_busy: false,
            latest_search_request_id: 0,
            latest_applied_search_request_id: 0,
            copy_flash_paths: Vec::new(),
            copy_flash_at: None,
        }
    }
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
    /// Stable hash of the multi-selection set.
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
            visible_rows_revision: browser.visible_rows_revision,
            selected_path_hash: selected_path.map(hash_path),
            loaded_path_hash: loaded_path.map(hash_path),
            selected_paths_hash: hash_paths(&browser.selected_paths),
            selection_anchor_visible: browser.selection_anchor_visible,
        }
    }
}

/// Hash one relative path into a stable scalar for marker-cache comparisons.
fn hash_path(path: &std::path::Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// Hash a multi-selection path list while preserving insertion order.
fn hash_paths(paths: &[PathBuf]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    paths.hash(&mut hasher);
    hasher.finish()
}

/// Holds the current similar-sounds query context.
#[derive(Clone, Debug)]
pub struct SimilarQuery {
    /// Sample id used as the similarity anchor.
    pub sample_id: String,
    /// Display label for the anchor sample.
    pub label: String,
    /// Entry indices in similarity order.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices` (0.0 = least similar, 1.0 = most similar).
    pub scores: Vec<f32>,
    /// Optional anchor index in the visible list.
    pub anchor_index: Option<usize>,
}

impl SimilarQuery {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }

    /// Return a normalized similarity strength for UI display.
    pub fn display_strength_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = *self.scores.get(position)?;
        // Use absolute scoring to match FocusedSimilarity and avoid misleading "relative best" highlighting.
        let normalized = ((score.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
        Some(normalized.powf(2.0))
    }
}

/// Highlight metadata for near-duplicate rows relative to the focused sample.
#[derive(Clone, Debug)]
pub struct FocusedSimilarity {
    /// Sample id used as the highlight anchor.
    pub sample_id: String,
    /// Entry indices for near-duplicate matches.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    pub scores: Vec<f32>,
    /// Absolute index of the focused sample, when known.
    pub anchor_index: Option<usize>,
}

impl FocusedSimilarity {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }
}

/// Visible list representation for the sample browser.
#[derive(Clone, Debug)]
pub enum VisibleRows {
    /// All rows are visible; total stores the count.
    All {
        /// Total number of rows.
        total: usize,
    },
    /// Only the provided indices are visible.
    List(Arc<[usize]>),
}

impl VisibleRows {
    /// Return the number of visible rows.
    pub fn len(&self) -> usize {
        match self {
            VisibleRows::All { total } => *total,
            VisibleRows::List(rows) => rows.len(),
        }
    }

    /// Copy a contiguous visible-window slice into `out`.
    ///
    /// The method clamps `start` and `len` to the valid range and clears
    /// `out` before appending results, so callers can reuse a pre-allocated
    /// buffer across frames.
    pub fn copy_window_into(&self, start: usize, len: usize, out: &mut Vec<usize>) {
        out.clear();
        if len == 0 {
            return;
        }

        match self {
            VisibleRows::All { total } => {
                if start >= *total {
                    return;
                }
                let end = start.saturating_add(len).min(*total);
                let count = end.saturating_sub(start);
                out.reserve(count);
                out.extend(start..end);
            }
            VisibleRows::List(rows) => {
                let end = start.saturating_add(len).min(rows.len());
                if start >= rows.len() {
                    return;
                }
                out.extend_from_slice(&rows[start..end]);
            }
        }
    }

    /// Map a visible row index to an absolute index.
    pub fn get(&self, row: usize) -> Option<usize> {
        match self {
            VisibleRows::All { total } => (row < *total).then_some(row),
            VisibleRows::List(rows) => rows.get(row).copied(),
        }
    }

    /// Reset the visible rows to an empty list.
    pub fn clear_to_list(&mut self) {
        *self = VisibleRows::List(Vec::new().into());
    }

    /// Iterate over visible absolute indices.
    pub fn iter(&self) -> Box<dyn Iterator<Item = usize> + '_> {
        match self {
            VisibleRows::All { total } => Box::new(0..*total),
            VisibleRows::List(rows) => Box::new(rows.iter().copied()),
        }
    }

    /// Find the visible position for an absolute index.
    pub fn position(&self, index: usize) -> Option<usize> {
        match self {
            VisibleRows::All { total } => (index < *total).then_some(index),
            VisibleRows::List(rows) => rows.iter().position(|i| *i == index),
        }
    }
}

/// Identifies a row inside one of the triage flag columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleBrowserIndex {
    /// Column containing the row.
    pub column: TriageFlagColumn,
    /// Row index within the column.
    pub row: usize,
}

/// Wav triage flag columns: trash, neutral, keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagColumn {
    /// Trash column.
    Trash,
    /// Neutral column.
    Neutral,
    /// Keep column.
    Keep,
}

/// Filter options for the single-column sample browser view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagFilter {
    /// Show all triage flags.
    All,
    /// Show keep-only rows.
    Keep,
    /// Show trash-only rows.
    Trash,
    /// Show untagged rows only.
    Untagged,
}

/// Sort modes for the sample browser list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserSort {
    /// Preserve the original list order.
    ListOrder,
    /// Sort by similarity score.
    Similarity,
    /// Sort by playback age ascending.
    PlaybackAgeAsc,
    /// Sort by playback age descending.
    PlaybackAgeDesc,
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

#[cfg(test)]
mod tests {
    use super::VisibleRows;

    #[test]
    fn visible_rows_all_copy_window_clamps_start_and_len() {
        let rows = VisibleRows::All { total: 7 };
        let mut out = Vec::new();
        rows.copy_window_into(4, 5, &mut out);
        assert_eq!(out, vec![4, 5, 6]);
    }

    #[test]
    fn visible_rows_list_copy_window_is_sliced() {
        let rows = VisibleRows::List(vec![10, 20, 30, 40, 50].into());
        let mut out = Vec::new();
        rows.copy_window_into(1, 3, &mut out);
        assert_eq!(out, vec![20, 30, 40]);
    }

    #[test]
    fn visible_rows_list_copy_window_respects_limits() {
        let rows = VisibleRows::List(vec![10, 20].into());
        let mut out = Vec::new();
        rows.copy_window_into(3, 2, &mut out);
        assert!(out.is_empty());
    }
}
