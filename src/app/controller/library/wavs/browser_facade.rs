use super::*;
use crate::app::controller::jobs::LoadedSimilarityQueryResult;
use crate::app::state::SampleBrowserTab;
use std::path::Path;

impl AppController {
    /// Select a wav row based on its path.
    pub fn select_wav_by_path(&mut self, path: &Path) {
        selection_ops::select_wav_by_path(self, path);
    }

    /// Select a wav row based on its path, optionally delaying the browser rebuild.
    pub fn select_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::select_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Focus a wav row by path without queueing audio/waveform loading.
    ///
    /// This supports high-frequency browser focus navigation where loading is
    /// committed separately by an explicit action.
    pub(crate) fn focus_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by path while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_path_preview_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_preview_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by absolute index while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_index_preview_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::focus_wav_by_index_preview_with_rebuild(self, index, rebuild);
    }

    /// Select a wav row by absolute index, optionally delaying browser list rebuild.
    pub(crate) fn select_wav_by_index_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::select_wav_by_index_with_rebuild(self, index, rebuild);
    }

    /// Map the current browser filter into a drop target tag for drag-and-drop retagging.
    pub fn triage_flag_drop_target(&self) -> TriageFlagColumn {
        selection_ops::triage_flag_drop_target(self)
    }

    /// Current tag of the selected wav, if any.
    pub fn selected_tag(&mut self) -> Option<crate::sample_sources::Rating> {
        selection_ops::selected_tag(self)
    }

    /// Apply a new browser filter and refresh visible rows.
    pub fn set_browser_filter(&mut self, filter: TriageFlagFilter) {
        browser_search::set_browser_filter(self, filter);
    }

    /// Apply a rating-level filter to the browser list (`-3..=3`, plus `4` for locked keeps).
    pub fn set_browser_rating_filter(&mut self, level: i8, additive: bool) {
        browser_search::set_browser_rating_filter(self, level, additive);
    }

    /// Apply a playback-age chip filter to the browser list.
    pub fn set_browser_playback_age_filter(
        &mut self,
        chip: crate::app::state::PlaybackAgeFilterChip,
        additive: bool,
    ) {
        browser_search::set_browser_playback_age_filter(self, chip, additive);
    }

    /// Invert one rating chip into the opposite rated bucket in the browser list.
    pub fn invert_browser_rating_filter(&mut self, level: i8) {
        browser_search::invert_browser_rating_filter(self, level);
    }

    /// Invert one playback-age chip into the opposite playback-age buckets in the browser list.
    pub fn invert_browser_playback_age_filter(
        &mut self,
        chip: crate::app::state::PlaybackAgeFilterChip,
    ) {
        browser_search::invert_browser_playback_age_filter(self, chip);
    }

    /// Clear any active rating-level filters in the browser list.
    pub fn clear_browser_rating_filter(&mut self) {
        browser_search::clear_browser_rating_filter(self);
    }

    /// Clear any active playback-age filters in the browser list.
    pub fn clear_browser_playback_age_filter(&mut self) {
        browser_search::clear_browser_playback_age_filter(self);
    }

    /// Toggle whether the browser shows only session-marked samples.
    pub fn toggle_browser_marked_filter(&mut self) {
        self.toggle_browser_marked_filter_action();
    }

    /// Toggle the session mark for the focused row or current multi-selection.
    pub fn toggle_browser_sample_mark(&mut self) {
        self.toggle_browser_sample_mark_action();
    }

    /// Apply a new sample browser sort mode and refresh visible rows.
    pub fn set_browser_sort(&mut self, sort: SampleBrowserSort) {
        browser_search::set_browser_sort(self, sort);
    }

    /// Request focus for the browser search input while keeping the browser context active.
    pub(crate) fn focus_browser_search(&mut self) {
        browser_search::focus_browser_search(self);
    }

    /// Clear browser-search focus while preserving the current query text.
    pub(crate) fn blur_browser_search(&mut self) {
        browser_search::blur_browser_search(self);
    }

    /// Apply a fuzzy search query to the browser and refresh visible rows.
    pub fn set_browser_search(&mut self, query: impl Into<String>) {
        browser_search::set_browser_search(self, query);
    }

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
                self.runtime.loaded_similarity_query_cache =
                    Some(crate::app::controller::library::wavs::similar::build_loaded_similarity_query_cache(&data));
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

    /// Build a library sample_id for the visible browser row.
    pub fn sample_id_for_visible_row(&mut self, row: usize) -> Result<String, String> {
        let source_id = self
            .selection_state
            .ctx
            .selected_source
            .clone()
            .ok_or_else(|| "No active source selected".to_string())?;
        let entry_index = self
            .ui
            .browser
            .viewport
            .visible
            .get(row)
            .ok_or_else(|| "Selected row is out of range".to_string())?;
        let entry = self
            .wav_entry(entry_index)
            .ok_or_else(|| "Sample entry missing".to_string())?;
        Ok(analysis_jobs::build_sample_id(
            source_id.as_str(),
            &entry.relative_path,
        ))
    }

    /// Build a library sample_id for the currently selected wav.
    pub fn selected_sample_id(&self) -> Option<String> {
        let source_id = self.selection_state.ctx.selected_source.as_ref()?;
        let path = self.sample_view.wav.selected_wav.as_ref()?;
        Some(analysis_jobs::build_sample_id(source_id.as_str(), path))
    }

    /// Focus the sample browser on a library sample_id without autoplay.
    pub fn focus_sample_from_map(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source_id = SourceId::from_string(source_id);
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
            self.select_source(Some(source_id.clone()));
        }
        self.focus_browser_context();
        self.ui.browser.selection.autoscroll = true;
        if !self.ui.browser.selection.selected_paths.is_empty() {
            self.clear_browser_selected_indices();
        }
        self.ui.browser.selection.selection_anchor_visible = None;
        self.selection_state.suppress_autoplay_once = true;
        self.select_wav_by_path(&relative_path);
        if let Some(row) = self.visible_row_for_path(&relative_path) {
            self.ui.browser.selection.selection_anchor_visible = Some(row);
        }
        Ok(())
    }

    /// Load waveform/audio for a given library sample_id without requiring browser selection.
    pub fn preview_sample_by_id(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|source| source.id.as_str() == source_id)
            .map(|source| SampleSource {
                id: source.id.clone(),
                root: source.root.clone(),
            })
            .ok_or_else(|| format!("Unknown source for sample_id: {sample_id}"))?;
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source.id) {
            self.select_source(Some(source.id.clone()));
        }
        self.sample_view.wav.selected_wav = Some(relative_path.clone());
        self.queue_audio_load_for(&source, &relative_path, AudioLoadIntent::Selection, None)
    }

    /// Select a wav by absolute index into the full wav list.
    pub fn select_wav_by_index(&mut self, index: usize) {
        selection_ops::select_wav_by_index(self, index);
    }

    /// Select a wav coming from the sample browser and clear collection focus.
    pub fn select_from_browser(&mut self, path: &Path) {
        selection_ops::select_from_browser(self, path);
    }

    /// Set triage tag for one sample path in the active source.
    pub(crate) fn set_sample_tag(
        &mut self,
        path: &Path,
        column: TriageFlagColumn,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag(self, path, column)
    }

    /// Set explicit triage tag for one sample path in a chosen source.
    pub(crate) fn set_sample_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        target_tag: crate::sample_sources::Rating,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag_for_source(self, source, path, target_tag, require_present)
    }

    /// Set explicit triage tag plus keep-lock state for one sample path in a chosen source.
    pub(crate) fn set_sample_tag_and_locked_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        target_tag: crate::sample_sources::Rating,
        locked: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag_and_locked_for_source(
            self,
            source,
            path,
            target_tag,
            locked,
            require_present,
        )
    }

    /// Update the keep-lock marker for a sample path within a specific source.
    pub(crate) fn set_sample_locked_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        locked: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_locked_for_source(self, source, path, locked, require_present)
    }

    /// Update the loop marker for a sample path within a specific source.
    pub(crate) fn set_sample_looped_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        looped: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_looped_for_source(self, source, path, looped, require_present)
    }
}
