//! Cached data for the controller, including databases and UI caches.

use super::super::{SampleSource, SourceDatabase, SourceDbError, SourceId, WavEntry};
use crate::app::controller::library::{source_folders, wavs};
use crate::app::state::FolderPaneId;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) struct WavCacheState {
    pub(crate) entries: HashMap<SourceId, WavEntriesState>,
}

impl WavCacheState {
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn insert_page(
        &mut self,
        source_id: SourceId,
        total: usize,
        page_size: usize,
        page_index: usize,
        entries: Vec<WavEntry>,
    ) {
        let cache = self
            .entries
            .entry(source_id)
            .or_insert_with(|| WavEntriesState::new(total, page_size));
        cache.total = total;
        cache.page_size = page_size;
        cache.insert_page(page_index, entries);
    }
}

pub(crate) struct LibraryCacheState {
    pub(crate) db: HashMap<SourceId, Rc<SourceDatabase>>,
    pub(crate) wav: WavCacheState,
}

impl LibraryCacheState {
    pub(crate) fn new() -> Self {
        Self {
            db: HashMap::new(),
            wav: WavCacheState::new(),
        }
    }

    /// Resolve or open the database for `source`, caching the handle.
    pub(crate) fn database_for(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        if let Some(existing) = self.db.get(&source.id) {
            return Ok(existing.clone());
        }
        let db = Rc::new(SourceDatabase::open_fast(&source.root)?);
        self.db.insert(source.id.clone(), db.clone());
        Ok(db)
    }
}

pub(crate) struct BrowserCacheState {
    pub(crate) labels: HashMap<SourceId, Vec<String>>,
    pub(crate) analysis_failures: HashMap<SourceId, HashMap<PathBuf, String>>,
    pub(crate) analysis_failures_pending: HashSet<SourceId>,
    /// Retained staged browser pipeline outputs keyed by revision fingerprints.
    pub(crate) pipeline: wavs::BrowserPipelineCache,
    pub(crate) search: wavs::BrowserSearchCache,
    pub(crate) features: HashMap<SourceId, FeatureCache>,
    pub(crate) bpm_values: HashMap<SourceId, HashMap<PathBuf, Option<f32>>>,
    pub(crate) durations: HashMap<SourceId, HashMap<PathBuf, f32>>,
}

/// Stable snapshot key for browser feature-cache rows aligned to the current wav list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeatureCacheKey {
    /// Number of wav entries the cache rows are aligned to.
    pub(crate) entries_len: usize,
    /// Stable hash of the ordered relative-path list backing the cache rows.
    pub(crate) entries_hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisJobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Canceled,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureStatus {
    pub(crate) has_features_v1: bool,
    pub(crate) has_embedding: bool,
    pub(crate) duration_seconds: Option<f32>,
    pub(crate) sr_used: Option<i64>,
    pub(crate) long_sample_mark: Option<bool>,
    pub(crate) analysis_status: Option<AnalysisJobStatus>,
}

/// Cached per-entry browser feature metadata aligned to one wav-entry snapshot.
#[derive(Clone, Debug)]
pub(crate) struct FeatureCache {
    pub(crate) key: FeatureCacheKey,
    pub(crate) rows: Vec<Option<FeatureStatus>>,
}

impl FeatureCache {
    /// Build an empty placeholder cache for one wav-entry snapshot key.
    pub(crate) fn empty(key: FeatureCacheKey) -> Self {
        Self {
            rows: vec![None; key.entries_len],
            key,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FolderBrowserCacheKey {
    pub(crate) pane: FolderPaneId,
    pub(crate) source_id: SourceId,
}

pub(crate) struct FolderBrowsersState {
    pub(crate) models: HashMap<FolderBrowserCacheKey, source_folders::FolderBrowserModel>,
    pub(crate) snapshots: HashMap<FolderBrowserCacheKey, source_folders::FolderTreeSnapshot>,
}

pub(crate) struct ControllerUiCacheState {
    pub(crate) browser: BrowserCacheState,
    pub(crate) folders: FolderBrowsersState,
}

impl ControllerUiCacheState {
    pub(crate) fn new() -> Self {
        Self {
            browser: BrowserCacheState {
                labels: HashMap::new(),
                analysis_failures: HashMap::new(),
                analysis_failures_pending: HashSet::new(),
                pipeline: wavs::BrowserPipelineCache::default(),
                search: wavs::BrowserSearchCache::default(),
                features: HashMap::new(),
                bpm_values: HashMap::new(),
                durations: HashMap::new(),
            },
            folders: FolderBrowsersState {
                models: HashMap::new(),
                snapshots: HashMap::new(),
            },
        }
    }
}

pub(crate) struct WavEntriesState {
    pub(crate) total: usize,
    pub(crate) page_size: usize,
    pub(crate) pages: HashMap<usize, Vec<WavEntry>>,
    pub(crate) lookup: HashMap<PathBuf, usize>,
    pub(crate) source_id: Option<SourceId>,
}

impl WavEntriesState {
    pub(crate) fn new(total: usize, page_size: usize) -> Self {
        Self {
            total,
            page_size: page_size.max(1),
            pages: HashMap::new(),
            lookup: HashMap::new(),
            source_id: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.total = 0;
        self.pages.clear();
        self.lookup.clear();
        self.source_id = None;
    }

    pub(crate) fn insert_page(&mut self, page_index: usize, entries: Vec<WavEntry>) {
        let offset = page_index * self.page_size;
        for (idx, entry) in entries.iter().enumerate() {
            self.insert_lookup(entry.relative_path.clone(), offset + idx);
        }
        self.pages.insert(page_index, entries);
    }

    pub(crate) fn entry(&self, index: usize) -> Option<&WavEntry> {
        let page_index = index / self.page_size;
        let in_page = index % self.page_size;
        self.pages
            .get(&page_index)
            .and_then(|page| page.get(in_page))
    }

    pub(crate) fn entry_mut(&mut self, index: usize) -> Option<&mut WavEntry> {
        let page_index = index / self.page_size;
        let in_page = index % self.page_size;
        self.pages
            .get_mut(&page_index)
            .and_then(|page| page.get_mut(in_page))
    }

    pub(crate) fn update_entry(&mut self, path: &Path, entry: WavEntry) -> bool {
        let normalized = path.to_string_lossy().replace('\\', "/");
        let Some(index) = self.lookup.get(Path::new(&normalized)).copied() else {
            return false;
        };
        let Some(slot) = self.entry_mut(index) else {
            return false;
        };
        *slot = entry;
        true
    }

    /// Insert one entry at the absolute index when the surrounding page window is loaded.
    ///
    /// Returns `true` when any loaded page content changed. If the insertion falls wholly
    /// outside the loaded page window, only `total` is updated and callers should treat the
    /// change as metadata-only for the current cache.
    pub(crate) fn insert_entry_at(&mut self, index: usize, entry: WavEntry) -> bool {
        self.total = self.total.saturating_add(1);
        let mut changed = false;
        let mut current_index = index;
        let mut carry = entry;
        loop {
            let page_index = current_index / self.page_size;
            let in_page = current_index % self.page_size;
            let Some(page) = self.pages.get_mut(&page_index) else {
                break;
            };
            let insert_at = in_page.min(page.len());
            page.insert(insert_at, carry);
            changed = true;
            if page.len() <= self.page_size {
                break;
            }
            let Some(displaced) = page.pop() else {
                break;
            };
            carry = displaced;
            current_index = (page_index + 1) * self.page_size;
        }
        if changed {
            self.lookup.clear();
            let mut lookup_entries = Vec::new();
            for (page_index, page) in &self.pages {
                let offset = page_index * self.page_size;
                for (idx, item) in page.iter().enumerate() {
                    lookup_entries.push((item.relative_path.clone(), offset + idx));
                }
            }
            for (path, index) in lookup_entries {
                self.insert_lookup(path, index);
            }
        }
        changed
    }

    pub(crate) fn insert_lookup(&mut self, path: PathBuf, index: usize) {
        let normalized = path.to_string_lossy().replace('\\', "/");
        self.lookup.insert(PathBuf::from(normalized), index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_lookup_normalizes_paths() {
        let mut cache = WavEntriesState::new(10, 10);

        // Insert with backslash
        cache.insert_lookup(PathBuf::from("foo\\bar.wav"), 1);

        // Should be found with forward slash
        assert_eq!(cache.lookup.get(Path::new("foo/bar.wav")), Some(&1));

        // Should be found with backslash (due to normalization on lookup/insert? No, insert normalizes key. Lookup must normalize query.)
        // We haven't updated lookup accessors on WavEntriesState itself other than update_entry.
        // Wait, update_entry calls lookup.get(path).
        // WavEntriesState::entry() accesses by index.

        // Let's verify internal storage is normalized (size is 1)
        assert_eq!(cache.lookup.len(), 1);
        assert!(cache.lookup.contains_key(Path::new("foo/bar.wav")));
    }

    #[test]
    fn test_update_entry_normalizes_lookup_key() {
        let mut cache = WavEntriesState::new(10, 10);

        // Mock entry existence
        cache.insert_page(
            0,
            vec![WavEntry {
                relative_path: PathBuf::from("foo/bar.wav"),
                file_size: 0,
                modified_ns: 0,
                content_hash: None,
                tag: crate::sample_sources::Rating::NEUTRAL,
                looped: false,
                locked: false,
                missing: false,
                last_played_at: None,
            }],
        );

        let new_entry = WavEntry {
            relative_path: PathBuf::from("foo/bar.wav"),
            file_size: 100,
            modified_ns: 100,
            content_hash: None,
            tag: crate::sample_sources::Rating::KEEP_1,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        };

        // Update using backslash path
        let success = cache.update_entry(Path::new("foo\\bar.wav"), new_entry);
        assert!(success, "Should find entry even with backslash path");

        // Verify update happened
        let entry = cache.entry(0).unwrap();
        assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_1);
    }
}
