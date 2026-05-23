use super::types::{
    BaseStageFingerprint, BrowserFeatureCacheSnapshot, CompactBrowserEntry, PlaybackAgeTokenCache,
};
use crate::sample_sources::WavEntry;
use std::sync::Arc;

mod triage;

/// Cache state for retained browser pipeline stages.
#[derive(Default)]
pub(crate) struct BrowserPipelineCache {
    /// Retained compact metadata aligned to absolute wav-entry indices.
    pub(super) compact_entries: Vec<CompactBrowserEntry>,
    /// Ordered feature-cache refresh snapshot aligned to the compact entries.
    pub(super) feature_cache_snapshot: Option<BrowserFeatureCacheSnapshot>,
    /// Fingerprint for the current base row snapshot.
    pub(super) base_fingerprint: Option<BaseStageFingerprint>,
    /// Absolute entry indices in source list order.
    pub(super) base_rows: Vec<usize>,
    /// Cached triage trash bucket in source list order.
    pub(crate) trash_rows: Vec<usize>,
    /// Cached triage neutral bucket in source list order.
    pub(crate) neutral_rows: Vec<usize>,
    /// Cached triage keep bucket in source list order.
    pub(crate) keep_rows: Vec<usize>,
    /// Fingerprint for the cached folder-filter acceptance map.
    pub(super) folder_accepts_fingerprint: Option<u64>,
    /// Whether the retained folder stage currently reflects active folder filters.
    pub(super) folder_accepts_active: bool,
    /// Cached folder-filter acceptance by absolute wav-entry index.
    pub(super) folder_accepts_by_index: Vec<bool>,
    /// Cached absolute indices accepted by the active folder filter.
    pub(super) folder_filtered_rows: Vec<usize>,
    /// Next playback-age rollover tokens cached for the current base snapshot by filter set.
    pub(super) playback_age_token_caches: Vec<PlaybackAgeTokenCache>,
    /// Fingerprint for the filtered stage rows.
    pub(super) filtered_fingerprint: Option<u64>,
    /// Filtered absolute entry indices.
    pub(super) filtered_rows: Vec<usize>,
    /// Fingerprint for the scored stage rows.
    pub(super) scored_fingerprint: Option<u64>,
    /// Scored rows in descending fuzzy-score order.
    pub(super) scored_rows: Vec<(usize, i64)>,
    /// Scratch lookup buffer used to sort similarity scores without per-build allocation.
    pub(super) similar_lookup_scratch: Vec<(usize, f32)>,
    /// Retained visible-row positions keyed by absolute entry index for the sorted stage.
    pub(super) sorted_row_positions: Vec<Option<usize>>,
    /// Fingerprint for the sorted stage rows.
    pub(super) sorted_fingerprint: Option<u64>,
    /// Sorted visible absolute entry indices, retained for cheap sharing.
    pub(super) sorted_rows: Arc<[usize]>,
}

impl BrowserPipelineCache {
    /// Return true when the base row snapshot has been built and can seed retained filters.
    pub(crate) fn has_base_snapshot(&self) -> bool {
        self.base_fingerprint.is_some()
    }

    /// Drop all staged fingerprints and vectors.
    pub(crate) fn invalidate(&mut self) {
        self.base_fingerprint = None;
        self.compact_entries.clear();
        self.feature_cache_snapshot = None;
        self.base_rows.clear();
        self.trash_rows.clear();
        self.neutral_rows.clear();
        self.keep_rows.clear();
        self.folder_accepts_fingerprint = None;
        self.folder_accepts_active = false;
        self.folder_accepts_by_index.clear();
        self.folder_filtered_rows.clear();
        self.playback_age_token_caches.clear();
        self.filtered_fingerprint = None;
        self.filtered_rows.clear();
        self.scored_fingerprint = None;
        self.scored_rows.clear();
        self.similar_lookup_scratch.clear();
        self.sorted_row_positions.clear();
        self.sorted_fingerprint = None;
        self.sorted_rows = Vec::new().into();
    }

    /// Return the retained feature-cache refresh snapshot for the current base rows.
    pub(crate) fn feature_cache_snapshot(&self) -> Option<BrowserFeatureCacheSnapshot> {
        self.feature_cache_snapshot.clone()
    }

    /// Update one retained playback-age value without rebuilding the whole base stage.
    pub(crate) fn update_playback_age(&mut self, index: usize, played_at: Option<i64>) -> bool {
        let Some(entry) = self.compact_entries.get_mut(index) else {
            return false;
        };
        if entry.last_played_at == played_at {
            return true;
        }
        entry.last_played_at = played_at;
        self.playback_age_token_caches.clear();
        self.invalidate_filter_and_sort_stages();
        true
    }

    /// Update one retained compact browser entry in place when only metadata changes.
    pub(crate) fn update_entry_metadata(&mut self, index: usize, entry: &WavEntry) -> bool {
        let Some(compact) = self.compact_entries.get_mut(index) else {
            return false;
        };
        let previous_tag = compact.tag;
        let previous_locked = compact.locked;
        let previous_last_played_at = compact.last_played_at;
        let previous_tag_named = compact.tag_named;
        compact.tag = entry.tag;
        compact.looped = entry.looped;
        compact.locked = entry.locked;
        compact.missing = entry.missing;
        compact.last_played_at = entry.last_played_at;
        compact.tag_named = entry.tag_named;
        if previous_tag != entry.tag {
            self.update_triage_partition_membership(index, previous_tag, entry.tag);
        }
        if previous_last_played_at != entry.last_played_at {
            self.playback_age_token_caches.clear();
        }
        if previous_tag != entry.tag
            || previous_locked != entry.locked
            || previous_last_played_at != entry.last_played_at
            || previous_tag_named != entry.tag_named
        {
            self.invalidate_filter_and_sort_stages();
        }
        true
    }

    /// Update one retained compact browser entry after a path or metadata change.
    pub(crate) fn update_entry_snapshot(&mut self, index: usize, entry: &WavEntry) -> bool {
        let Some(compact) = self.compact_entries.get_mut(index) else {
            return false;
        };
        let previous_path = compact.relative_path.clone();
        let previous_tag = compact.tag;
        let previous_locked = compact.locked;
        let previous_last_played_at = compact.last_played_at;
        let previous_tag_named = compact.tag_named;
        compact.relative_path = entry.relative_path.clone();
        compact.tag = entry.tag;
        compact.looped = entry.looped;
        compact.locked = entry.locked;
        compact.missing = entry.missing;
        compact.last_played_at = entry.last_played_at;
        compact.tag_named = entry.tag_named;
        if previous_tag != entry.tag {
            self.update_triage_partition_membership(index, previous_tag, entry.tag);
        }
        if previous_path != entry.relative_path
            && let Some(snapshot) = self.feature_cache_snapshot.as_mut()
        {
            let paths = Arc::make_mut(&mut snapshot.entry_paths);
            if index < paths.len() {
                paths[index] = entry.relative_path.clone();
                snapshot.key =
                    crate::app::controller::library::wavs::feature_cache::feature_cache_key_for_paths(
                        paths,
                    );
            }
        }
        if previous_path != entry.relative_path || previous_last_played_at != entry.last_played_at {
            self.playback_age_token_caches.clear();
        }
        if previous_path != entry.relative_path
            || previous_tag != entry.tag
            || previous_locked != entry.locked
            || previous_last_played_at != entry.last_played_at
            || previous_tag_named != entry.tag_named
        {
            self.invalidate_filter_and_sort_stages();
        }
        true
    }

    /// Sync the retained base-stage revision after metadata-only DB writes finish.
    pub(crate) fn sync_source_revision(&mut self, source_revision: Option<u64>) {
        if let Some(fingerprint) = self.base_fingerprint.as_mut() {
            fingerprint.source_revision = source_revision;
        }
    }

    /// Prepare reusable similarity-score scratch for `capacity` sparse score entries.
    pub(super) fn prepare_similar_lookup_scratch(&mut self, capacity: usize) {
        self.similar_lookup_scratch.clear();
        if self.similar_lookup_scratch.capacity() < capacity {
            self.similar_lookup_scratch
                .reserve(capacity.saturating_sub(self.similar_lookup_scratch.capacity()));
        }
    }

    /// Rebuild the sorted-stage absolute-index lookup table for the latest visible rows.
    pub(super) fn rebuild_sorted_row_positions(&mut self, sorted_rows: &[usize]) {
        self.sorted_row_positions.clear();
        self.sorted_row_positions
            .resize(self.compact_entries.len(), None);
        for (visible_row, index) in sorted_rows.iter().copied().enumerate() {
            if let Some(slot) = self.sorted_row_positions.get_mut(index) {
                *slot = Some(visible_row);
            }
        }
    }

    /// Resolve one visible-row position from the retained sorted-stage lookup table.
    pub(super) fn sorted_visible_position(&self, index: usize) -> Option<usize> {
        self.sorted_row_positions.get(index).copied().flatten()
    }

    pub(super) fn refresh_base_partitions(&mut self) {
        self.base_rows.clear();
        self.base_rows.reserve(self.compact_entries.len());
        self.trash_rows.clear();
        self.neutral_rows.clear();
        self.keep_rows.clear();
        for (index, entry) in self.compact_entries.iter().enumerate() {
            self.base_rows.push(index);
            if entry.tag.is_trash() {
                self.trash_rows.push(index);
            } else if entry.tag.is_keep() {
                self.keep_rows.push(index);
            } else {
                self.neutral_rows.push(index);
            }
        }
        self.playback_age_token_caches.clear();
        self.filtered_fingerprint = None;
        self.scored_fingerprint = None;
        self.sorted_row_positions.clear();
        self.sorted_fingerprint = None;
        self.folder_accepts_fingerprint = None;
        self.folder_accepts_active = false;
        self.folder_accepts_by_index.clear();
        self.folder_filtered_rows.clear();
    }

    fn invalidate_filter_and_sort_stages(&mut self) {
        self.filtered_fingerprint = None;
        self.scored_fingerprint = None;
        self.sorted_row_positions.clear();
        self.sorted_fingerprint = None;
    }
}
