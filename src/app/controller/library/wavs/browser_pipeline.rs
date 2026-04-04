use super::*;

mod base_stage;
mod folder_stage;
/// Shared stage helper functions for sort/filter/hash operations.
mod helpers;
#[cfg(test)]
mod tests;
mod visible_rows;

use self::base_stage::ensure_base_stage;
pub(crate) use self::visible_rows::build_visible_rows;
use crate::app::controller::FeatureCacheKey;
use crate::sample_sources::SourceId;
use crate::sample_sources::{Rating, WavEntry};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// Cache state for retained browser pipeline stages.
#[derive(Default)]
pub(crate) struct BrowserPipelineCache {
    /// Retained compact metadata aligned to absolute wav-entry indices.
    compact_entries: Vec<CompactBrowserEntry>,
    /// Ordered feature-cache refresh snapshot aligned to the compact entries.
    feature_cache_snapshot: Option<BrowserFeatureCacheSnapshot>,
    /// Fingerprint for the current base row snapshot.
    base_fingerprint: Option<BaseStageFingerprint>,
    /// Absolute entry indices in source list order.
    base_rows: Vec<usize>,
    /// Cached triage trash bucket in source list order.
    pub(crate) trash_rows: Vec<usize>,
    /// Cached triage neutral bucket in source list order.
    pub(crate) neutral_rows: Vec<usize>,
    /// Cached triage keep bucket in source list order.
    pub(crate) keep_rows: Vec<usize>,
    /// Fingerprint for the cached folder-filter acceptance map.
    folder_accepts_fingerprint: Option<u64>,
    /// Cached folder-filter acceptance by absolute wav-entry index.
    folder_accepts_by_index: Vec<bool>,
    /// Next playback-age rollover token cached for the current base snapshot and filter set.
    playback_age_token_cache: Option<PlaybackAgeTokenCache>,
    /// Fingerprint for the filtered stage rows.
    filtered_fingerprint: Option<u64>,
    /// Filtered absolute entry indices.
    filtered_rows: Vec<usize>,
    /// Fingerprint for the scored stage rows.
    scored_fingerprint: Option<u64>,
    /// Scored rows in descending fuzzy-score order.
    scored_rows: Vec<(usize, i64)>,
    /// Scratch lookup buffer used to sort similarity scores without per-build allocation.
    similar_lookup_scratch: Vec<Option<f32>>,
    /// Fingerprint for the sorted stage rows.
    sorted_fingerprint: Option<u64>,
    /// Sorted visible absolute entry indices, retained for cheap sharing.
    sorted_rows: Arc<[usize]>,
}

impl BrowserPipelineCache {
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
        self.folder_accepts_by_index.clear();
        self.playback_age_token_cache = None;
        self.filtered_fingerprint = None;
        self.filtered_rows.clear();
        self.scored_fingerprint = None;
        self.scored_rows.clear();
        self.similar_lookup_scratch.clear();
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
        self.playback_age_token_cache = None;
        self.filtered_fingerprint = None;
        self.scored_fingerprint = None;
        self.sorted_fingerprint = None;
        true
    }

    /// Prepare reusable similarity-score scratch for `len` absolute row slots.
    fn prepare_similar_lookup_scratch(&mut self, len: usize) {
        self.similar_lookup_scratch.clear();
        self.similar_lookup_scratch.resize(len, None);
    }
}

/// Compact synchronous browser-filter metadata aligned to absolute wav-entry indices.
#[derive(Clone)]
struct CompactBrowserEntry {
    relative_path: PathBuf,
    tag: Rating,
    looped: bool,
    locked: bool,
    missing: bool,
    last_played_at: Option<i64>,
}

impl CompactBrowserEntry {
    /// Build the compact retained entry payload required by the sync browser pipeline.
    fn from_wav_entry(entry: WavEntry) -> Self {
        Self {
            relative_path: entry.relative_path,
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
        }
    }
}

/// Borrowed browser-row metadata used by page-load-free projection helpers.
#[derive(Clone, Copy)]
pub(crate) struct BrowserProjectionEntryRef<'a> {
    /// Stable source-relative path for labels and cached metadata.
    pub(crate) relative_path: &'a Path,
    /// Current triage rating for the sample.
    pub(crate) tag: Rating,
    /// Whether the sample should render the loop badge.
    pub(crate) looped: bool,
    /// Whether the sample is locked as a keep.
    pub(crate) locked: bool,
    /// Whether the sample is missing on disk.
    pub(crate) missing: bool,
    /// Last-played timestamp used for playback-age buckets.
    pub(crate) last_played_at: Option<i64>,
}

impl<'a> BrowserProjectionEntryRef<'a> {
    /// Borrow one projection entry from the retained compact browser snapshot.
    fn from_compact_entry(entry: &'a CompactBrowserEntry) -> Self {
        Self {
            relative_path: entry.relative_path.as_path(),
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
        }
    }

    /// Borrow one projection entry from an already loaded wav-entry page.
    fn from_loaded_entry(entry: &'a WavEntry) -> Self {
        Self {
            relative_path: entry.relative_path.as_path(),
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
        }
    }
}

/// Retained ordered path snapshot used to refresh browser feature metadata.
#[derive(Clone)]
pub(crate) struct BrowserFeatureCacheSnapshot {
    /// Stable key for the current ordered path list.
    pub(crate) key: FeatureCacheKey,
    /// Ordered relative paths aligned to absolute wav-entry indices.
    pub(crate) entry_paths: Arc<[PathBuf]>,
}

impl AppController {
    /// Return one browser entry snapshot without forcing a wav-page load.
    ///
    /// Hot browser projection paths should prefer the retained pipeline snapshot
    /// and only fall back to entries that are already loaded in the page cache.
    pub(crate) fn browser_projection_entry(
        &self,
        index: usize,
    ) -> Option<BrowserProjectionEntryRef<'_>> {
        self.ui_cache
            .browser
            .pipeline
            .compact_entries
            .get(index)
            .map(BrowserProjectionEntryRef::from_compact_entry)
            .or_else(|| {
                self.wav_entries
                    .entry(index)
                    .map(BrowserProjectionEntryRef::from_loaded_entry)
            })
    }

    /// Return the retained browser feature-cache snapshot for the active source.
    pub(crate) fn current_browser_feature_cache_snapshot(
        &mut self,
    ) -> Option<BrowserFeatureCacheSnapshot> {
        ensure_base_stage(self);
        self.ui_cache.browser.pipeline.feature_cache_snapshot()
    }
}

/// Stable identity for the stage-A base snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BaseStageFingerprint {
    source_id: Option<SourceId>,
    source_revision: Option<u64>,
    entries_len: usize,
}

/// Cached next playback-age rollover token for one retained base snapshot and chip set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlaybackAgeTokenCache {
    base_fingerprint_hash: u64,
    filter_hash: u64,
    token: Option<i64>,
}
