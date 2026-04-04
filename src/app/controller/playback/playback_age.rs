//! Deferred playback-age persistence helpers.

use super::*;
use crate::app::controller::jobs::SourceMetadataMutationOp;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::controller::state::runtime::MetadataRollback;
use std::path::Path;
use std::time::Instant;

impl AppController {
    /// Record playback for the currently loaded audio, updating caches and UI.
    pub(crate) fn record_loaded_audio_playback(&mut self) {
        let Some(audio) = self.sample_view.wav.loaded_audio.as_ref() else {
            return;
        };
        let source_id = audio.source_id.clone();
        let root = audio.root.clone();
        let relative_path = audio.relative_path.clone();
        let played_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.audio.pending_age_update = Some(PendingAgeUpdate {
            source_id: source_id.clone(),
            root,
            relative_path: relative_path.clone(),
            played_at,
        });

        if self.selection_state.ctx.selected_source.as_ref() == Some(&source_id)
            && let Some(index) = self.wav_index_for_path(&relative_path)
        {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(entry) = self.wav_entries.entry_mut(index) {
                entry.last_played_at = Some(played_at);
            }
            self.ui_cache.browser.pipeline.invalidate();
            self.mark_browser_row_metadata_projection_revision_dirty();
            if self.ui.browser.search.sort == crate::app::state::SampleBrowserSort::PlaybackAgeAsc
                || self.ui.browser.search.sort
                    == crate::app::state::SampleBrowserSort::PlaybackAgeDesc
                || !self.ui.browser.search.playback_age_filter.is_empty()
            {
                self.mark_browser_search_projection_revision_dirty();
                if self.should_dispatch_browser_search_async() {
                    self.dispatch_search_job();
                } else {
                    self.rebuild_browser_lists();
                }
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(&source_id)
            && let Some(index) = cache.lookup.get(&relative_path).copied()
            && let Some(entry) = cache.entry_mut(index)
        {
            entry.last_played_at = Some(played_at);
        }
    }

    /// Defer pending playback-age persistence when focus changes away from a path.
    pub(crate) fn defer_pending_age_update_commit_if_path_changes(&mut self, next_path: &Path) {
        let should_defer = self
            .audio
            .pending_age_update
            .as_ref()
            .is_some_and(|update| update.relative_path != next_path);
        if !should_defer {
            return;
        }
        self.runtime.pending_age_update_commit = self.audio.pending_age_update.take();
        self.runtime.pending_age_update_commit_not_before =
            Some(Instant::now() + DEFERRED_PLAYBACK_AGE_COMMIT_DELAY);
    }

    /// Flush any deferred playback-age update persistence request.
    pub(crate) fn flush_pending_age_update_commit(&mut self) {
        if self
            .runtime
            .pending_age_update_commit_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_age_update_commit_not_before = None;
        let Some(update) = self.runtime.pending_age_update_commit.take() else {
            return;
        };
        self.commit_pending_age_update_value(update);
    }

    /// Return true when deferred playback-age persistence is queued.
    pub(crate) fn has_pending_age_update_commit(&self) -> bool {
        self.runtime.pending_age_update_commit.is_some()
    }

    /// Commit any pending playback age update to the database and refresh the UI.
    pub fn commit_pending_age_update(&mut self) {
        let Some(update) = self.audio.pending_age_update.take() else {
            return;
        };
        self.commit_pending_age_update_value(update);
    }

    /// Persist one playback-age update payload and refresh browser lists when needed.
    fn commit_pending_age_update_value(&mut self, update: PendingAgeUpdate) {
        let source = SampleSource {
            id: update.source_id.clone(),
            root: update.root,
        };
        let before_last_played_at = self
            .wav_index_for_path(&update.relative_path)
            .and_then(|index| self.wav_entries.entry(index))
            .and_then(|entry| entry.last_played_at);
        self.queue_metadata_mutation(
            &source,
            vec![SourceMetadataMutationOp::SetLastPlayedAt {
                relative_path: update.relative_path.clone(),
                played_at: update.played_at,
            }],
            Vec::new(),
            vec![MetadataRollback::LastPlayedAt {
                relative_path: update.relative_path.clone(),
                before_last_played_at,
                expected_last_played_at: Some(update.played_at),
            }],
            true,
        );
    }
}
