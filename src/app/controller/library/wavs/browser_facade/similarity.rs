use super::super::*;
use crate::app::controller::jobs::LoadedSimilarityQueryResult;
use crate::app::state::SampleBrowserTab;
use std::path::Path;

impl AppController {
    /// Filter the browser to show similar samples for the chosen visible row.
    pub fn find_similar_for_visible_row(&mut self, row: usize) -> Result<(), String> {
        similar::find_similar_for_visible_row(self, row)
    }

    /// Refresh similarity-sort state for the loaded sample, disabling sort on failure.
    pub(crate) fn refresh_similarity_sort_for_loaded_sample(&mut self) {
        if let Err(err) = similar::queue_loaded_similarity_query_refresh(self) {
            similar::disable_similarity_sort(self);
            self.set_status(err, StatusTone::Warning);
        }
    }

    /// Sort the browser by similarity to the loaded sample.
    pub fn enable_loaded_similarity_sort(&mut self) -> Result<(), String> {
        similar::enable_loaded_similarity_sort(self)
    }

    /// Apply one async follow-loaded similarity query if it still matches the active waveform.
    pub(crate) fn handle_loaded_similarity_query_built(
        &mut self,
        result: LoadedSimilarityQueryResult,
    ) {
        let Some(pending) = self.runtime.pending_loaded_similarity_query.as_ref() else {
            return;
        };
        if pending.request_id != result.request_id
            || pending.source_id != result.source_id
            || pending.relative_path != result.relative_path
            || pending.key != result.key
        {
            return;
        }
        self.runtime.pending_loaded_similarity_query = None;
        let still_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == result.source_id && audio.relative_path == result.relative_path
            });
        if !still_loaded
            || !self.ui.browser.search.similarity_sort_follow_loaded
            || self.ui.browser.search.sort != SampleBrowserSort::Similarity
        {
            return;
        }
        let key_matches_current_snapshot = self
            .current_browser_feature_cache_snapshot()
            .is_some_and(|snapshot| snapshot.key == result.key);
        if !key_matches_current_snapshot {
            self.refresh_similarity_sort_for_loaded_sample();
            return;
        }
        match result.result {
            Ok(data) => {
                self.runtime.loaded_similarity_query_cache = Some(
                    crate::app::controller::library::wavs::similar::build_loaded_similarity_query_cache(
                        &data,
                    ),
                );
                self.ui.browser.search.similar_query = Some(data.query);
                if self.should_dispatch_browser_search_async() {
                    self.dispatch_search_job();
                } else {
                    self.rebuild_browser_lists();
                }
            }
            Err(err) => {
                similar::disable_similarity_sort(self);
                self.set_status(err, StatusTone::Warning);
            }
        }
    }

    /// Disable similarity-based sorting and restore list order.
    pub fn disable_similarity_sort(&mut self) {
        similar::disable_similarity_sort(self);
    }

    /// Filter the browser to show near-duplicate samples for the chosen visible row.
    pub fn find_duplicates_for_visible_row(&mut self, row: usize) -> Result<(), String> {
        similar::find_duplicates_for_visible_row(self, row)
    }

    /// Filter the browser to show similar samples for a specific library sample_id.
    pub fn find_similar_for_sample_id(&mut self, sample_id: &str) -> Result<(), String> {
        similar::find_similar_for_sample_id(self, sample_id)
    }

    /// Filter the browser to show similar samples for an external audio clip.
    pub fn find_similar_for_audio_path(&mut self, path: &Path) -> Result<(), String> {
        similar::find_similar_for_audio_path(self, path)
    }

    /// Clear any active similar-sounds filter.
    pub fn clear_similar_filter(&mut self) {
        similar::clear_similar_filter(self);
    }

    /// Toggle similarity filtering for the focused browser sample.
    ///
    /// This normalizes browser tab state first so both native dispatchers and
    /// hotkeys rely on the same focused-sample similarity contract.
    pub(crate) fn toggle_find_similar_focused_sample(&mut self) {
        if self.clear_browser_duplicate_cleanup() {
            self.set_status("Duplicate cleanup off", StatusTone::Info);
        }
        if matches!(self.ui.browser.active_tab, SampleBrowserTab::Map) {
            self.set_browser_tab(false);
        }
        let Some(row) = self.focused_browser_row() else {
            self.set_status("Focus a sample to find similar", StatusTone::Info);
            return;
        };
        let focused_sample_id = self.sample_id_for_visible_row(row).ok();
        let query_matches_focus = self
            .ui
            .browser
            .search
            .similar_query
            .as_ref()
            .zip(focused_sample_id.as_deref())
            .is_some_and(|(query, focused_sample_id)| query.sample_id == focused_sample_id);
        if query_matches_focus {
            self.clear_similar_filter();
            return;
        }
        if let Err(err) = self.find_similar_for_visible_row(row) {
            self.set_status(format!("Find similar failed: {err}"), StatusTone::Error);
        }
    }
}
