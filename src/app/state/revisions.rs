//! Canonical revision counters used by native projection invalidation.

/// Monotonic revision counters for projection-sensitive UI slices.
///
/// Controller frame prep updates these counters whenever the corresponding
/// source fields change. Native bridge cache keys then depend on these scalar
/// revisions instead of hashing container payloads every pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiProjectionRevisions {
    /// Status text/tone revision.
    pub status: u64,
    /// Folder-search query revision.
    pub folder_search: u64,
    /// Browser-search query revision.
    pub browser_search: u64,
    /// Map selection sample id revision.
    pub map_selection: u64,
    /// Map hovered sample id revision.
    pub map_hover: u64,
    /// Map dataset identity revision (`umap_version` and cached source/version ids).
    pub map_dataset: u64,
    /// Map query-bounds revision.
    pub map_query: u64,
    /// Update panel text/status revision.
    pub update: u64,
    /// Loaded wav path revision.
    pub loaded_wav: u64,
}

impl UiProjectionRevisions {
    /// Bump one revision counter using wrapping arithmetic.
    pub fn bump(counter: &mut u64) {
        *counter = counter.wrapping_add(1);
    }
}
