use super::*;

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

    /// Return a stable fingerprint for the retained browser path ordering.
    pub(crate) fn browser_search_path_fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for entry in &self.ui_cache.browser.pipeline.compact_entries {
            std::hash::Hash::hash(&entry.relative_path, &mut hasher);
        }
        std::hash::Hasher::finish(&hasher)
    }
}
