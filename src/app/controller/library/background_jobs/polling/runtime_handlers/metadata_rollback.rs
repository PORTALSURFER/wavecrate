use super::*;
use crate::app::controller::state::runtime::MetadataRollback;

impl AppController {
    pub(in crate::app::controller::library::background_jobs::polling::runtime_handlers) fn rollback_metadata_mutation(
        &mut self,
        source_id: &SourceId,
        rollback: &[MetadataRollback],
    ) {
        let active_source_matches =
            self.selection_state.ctx.selected_source.as_ref() == Some(source_id);
        for entry in rollback {
            self.rollback_one_metadata_entry(source_id, entry, active_source_matches);
        }
        if !active_source_matches {
            return;
        }
        self.ui_cache.browser.pipeline.invalidate();
        self.mark_browser_row_metadata_projection_revision_dirty();
        self.mark_browser_search_projection_revision_dirty();
        if self.should_dispatch_browser_search_async() {
            self.dispatch_search_job();
        } else {
            self.rebuild_browser_lists();
        }
    }

    fn rollback_one_metadata_entry(
        &mut self,
        source_id: &SourceId,
        entry: &MetadataRollback,
        active_source_matches: bool,
    ) {
        match entry {
            MetadataRollback::TagAndLocked {
                relative_path,
                before_tag,
                before_locked,
                expected_tag,
                expected_locked,
            } => self.rollback_tag_and_locked_metadata(
                source_id,
                relative_path,
                *before_tag,
                *before_locked,
                *expected_tag,
                *expected_locked,
                active_source_matches,
            ),
            MetadataRollback::Looped {
                relative_path,
                intent_id,
                before_looped,
                expected_looped,
            } => self.rollback_looped_metadata(
                source_id,
                relative_path,
                *intent_id,
                *before_looped,
                *expected_looped,
                active_source_matches,
            ),
            MetadataRollback::SoundType {
                relative_path,
                before_sound_type,
                expected_sound_type,
            } => self.rollback_sound_type_metadata(
                source_id,
                relative_path,
                *before_sound_type,
                *expected_sound_type,
                active_source_matches,
            ),
            MetadataRollback::UserTag {
                relative_path,
                before_user_tag,
                expected_user_tag,
            } => self.rollback_user_tag_metadata(
                source_id,
                relative_path,
                before_user_tag,
                expected_user_tag,
                active_source_matches,
            ),
            MetadataRollback::NormalTag {
                relative_path,
                normalized_text,
                display_label,
                before_present,
                expected_present,
            } => self.rollback_normal_tag_metadata(
                source_id,
                relative_path,
                normalized_text,
                display_label,
                *before_present,
                *expected_present,
                active_source_matches,
            ),
            MetadataRollback::LastPlayedAt {
                relative_path,
                before_last_played_at,
                expected_last_played_at,
            } => self.rollback_last_played_metadata(
                source_id,
                relative_path,
                *before_last_played_at,
                *expected_last_played_at,
                active_source_matches,
            ),
            MetadataRollback::Bpm {
                relative_path,
                before_bpm,
                expected_bpm,
            } => self.rollback_bpm_metadata(source_id, relative_path, *before_bpm, *expected_bpm),
        }
    }

    fn rollback_tag_and_locked_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_tag: Rating,
        before_locked: bool,
        expected_tag: Rating,
        expected_locked: bool,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.tag == expected_tag
                && wav.locked == expected_locked
            {
                wav.tag = before_tag;
                wav.locked = before_locked;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.tag == expected_tag
            && wav.locked == expected_locked
        {
            wav.tag = before_tag;
            wav.locked = before_locked;
        }
    }

    fn rollback_looped_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
        before_looped: bool,
        expected_looped: bool,
        active_source_matches: bool,
    ) {
        let relative_path = self.resolve_looped_rollback_path(source_id, relative_path, intent_id);
        if !self
            .runtime
            .source_lane
            .mutations
            .looped_metadata_intent_matches(source_id, &relative_path, intent_id)
        {
            return;
        }
        if active_source_matches && let Some(index) = self.wav_index_for_path(&relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.looped == expected_looped
            {
                wav.looped = before_looped;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(&relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.looped == expected_looped
        {
            wav.looped = before_looped;
        }
        self.runtime
            .source_lane
            .mutations
            .finish_looped_metadata_intent(source_id, &relative_path, intent_id);
    }

    fn rollback_sound_type_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_sound_type: Option<crate::sample_sources::SampleSoundType>,
        expected_sound_type: Option<crate::sample_sources::SampleSoundType>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.sound_type == expected_sound_type
            {
                wav.sound_type = before_sound_type;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.sound_type == expected_sound_type
        {
            wav.sound_type = before_sound_type;
        }
    }

    fn rollback_user_tag_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_user_tag: &Option<String>,
        expected_user_tag: &Option<String>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.user_tag == *expected_user_tag
            {
                wav.user_tag = before_user_tag.clone();
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.user_tag == *expected_user_tag
        {
            wav.user_tag = before_user_tag.clone();
        }
    }

    fn rollback_last_played_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_last_played_at: Option<i64>,
        expected_last_played_at: Option<i64>,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index)
                && wav.last_played_at == expected_last_played_at
            {
                wav.last_played_at = before_last_played_at;
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
            && wav.last_played_at == expected_last_played_at
        {
            wav.last_played_at = before_last_played_at;
        }
    }

    fn rollback_bpm_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        before_bpm: Option<f32>,
        expected_bpm: Option<f32>,
    ) {
        let cache = self
            .ui_cache
            .browser
            .bpm_values
            .entry(source_id.clone())
            .or_default();
        if cache.get(relative_path).copied().flatten() == expected_bpm {
            cache.insert(relative_path.to_path_buf(), before_bpm);
        }
    }
}
