use super::*;
use crate::app::controller::jobs::LoadedSimilarityQueryResult;
use crate::app::state::SampleBrowserTab;
use std::path::Path;

impl AppController {
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

    /// Update the sound-type metadata for a sample path within a specific source.
    pub(crate) fn set_sample_sound_type_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        sound_type: Option<crate::sample_sources::SampleSoundType>,
    ) -> Result<(), String> {
        selection_ops::set_sample_sound_type_for_source(self, source, path, sound_type)
    }

    /// Update the custom user tag for a sample path within a specific source.
    pub(crate) fn set_sample_user_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        user_tag: Option<String>,
    ) -> Result<(), String> {
        selection_ops::set_sample_user_tag_for_source(self, source, path, user_tag)
    }

    /// Assign one normal library tag for a sample path within a specific source.
    pub(crate) fn apply_normal_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        label: &str,
    ) -> Result<(), String> {
        selection_ops::apply_normal_tag_for_source(self, source, path, label)
    }

    /// Remove one normal library tag assignment for a sample path within a specific source.
    pub(crate) fn remove_normal_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        label: &str,
    ) -> Result<(), String> {
        selection_ops::remove_normal_tag_for_source(self, source, path, label)
    }

    /// Return normal library tags for a sample path, using the controller cache when available.
    pub(crate) fn normal_tags_for_path(
        &mut self,
        source: &SampleSource,
        path: &Path,
    ) -> Result<Vec<crate::sample_sources::db::SourceTag>, String> {
        selection_ops::normal_tags_for_path(self, source, path)
    }

    /// Summarize one normal tag across a focused/selected target set.
    pub(crate) fn normal_tag_state_for_source(
        &mut self,
        source: &SampleSource,
        paths: &[std::path::PathBuf],
        label: &str,
    ) -> Result<crate::app_core::actions::NativeBrowserTagState, String> {
        selection_ops::normal_tag_state_for_source(self, source, paths, label)
    }

    /// Toggle the browser-local metadata sidebar inside the list tab.
    pub(crate) fn toggle_browser_tag_sidebar(&mut self) {
        let is_list_tab = matches!(self.ui.browser.active_tab, SampleBrowserTab::List);
        self.ui.browser.tag_sidebar_open = is_list_tab && !self.ui.browser.tag_sidebar_open;
    }

    /// Toggle whether sidebar metadata edits should auto-rename edited samples.
    pub(crate) fn toggle_browser_tag_sidebar_auto_rename(&mut self) {
        self.ui.browser.tag_sidebar_auto_rename = !self.ui.browser.tag_sidebar_auto_rename;
        let label = if self.ui.browser.tag_sidebar_auto_rename {
            "Auto rename on"
        } else {
            "Auto rename off"
        };
        self.set_status(label, StatusTone::Info);
    }

    /// Store the current draft value for the browser metadata custom-tag input.
    pub(crate) fn set_browser_tag_sidebar_input(&mut self, value: String) {
        self.ui.browser.tag_sidebar_input = value;
    }

    /// Apply the current custom-tag input draft to focused/selected browser rows.
    pub(crate) fn commit_browser_tag_sidebar_input(&mut self) -> Result<(), String> {
        let value = self.ui.browser.tag_sidebar_input.clone();
        self.apply_browser_tag_sidebar_normal_tag(&value)
    }

    /// Apply one playback-type value to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_looped(&mut self, looped: bool) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let target_paths = self.browser_tag_sidebar_target_paths();
        for path in &target_paths {
            self.set_sample_looped_for_source(&source, &path, looped, false)?;
        }
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    /// Apply one sound-type value to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_sound_type(
        &mut self,
        sound_type: Option<crate::sample_sources::SampleSoundType>,
    ) -> Result<(), String> {
        match sound_type {
            Some(sound_type) => self.apply_browser_tag_sidebar_normal_tag(sound_type.token()),
            None => Ok(()),
        }
    }

    /// Apply or clear the single custom user tag for the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_user_tag(
        &mut self,
        user_tag: Option<String>,
    ) -> Result<(), String> {
        let Some(user_tag) = user_tag else {
            return Ok(());
        };
        self.apply_browser_tag_sidebar_normal_tag(&user_tag)
    }

    /// Assign one normal tag to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_normal_tag(
        &mut self,
        label: &str,
    ) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let resolved_label = self.resolve_browser_normal_tag_label(&source, label)?;
        let target_paths = self.browser_tag_sidebar_target_paths();
        for path in &target_paths {
            self.apply_normal_tag_for_source(&source, &path, &resolved_label)?;
        }
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    /// Remove one normal tag from the focused/selected browser rows.
    pub(crate) fn remove_browser_tag_sidebar_normal_tag(
        &mut self,
        label: &str,
    ) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let target_paths = self.browser_tag_sidebar_target_paths();
        for path in &target_paths {
            self.remove_normal_tag_for_source(&source, &path, label)?;
        }
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    fn resolve_browser_normal_tag_label(
        &mut self,
        source: &SampleSource,
        label: &str,
    ) -> Result<String, String> {
        let trimmed = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            return Err(String::from("Tag label cannot be empty"));
        }
        let matches = self
            .database_for(source)
            .map_err(|err| err.to_string())?
            .search_tags(&trimmed, 1)
            .map_err(|err| err.to_string())?;
        Ok(matches
            .first()
            .map(|usage| usage.tag.display_label.clone())
            .unwrap_or(trimmed))
    }

    fn auto_rename_after_tag_sidebar_change(
        &mut self,
        target_paths: &[std::path::PathBuf],
    ) -> Result<(), String> {
        if !self.ui.browser.tag_sidebar_auto_rename || target_paths.is_empty() {
            return Ok(());
        }
        self.browser()
            .auto_rename_browser_sample_paths_action(target_paths)
    }
}
