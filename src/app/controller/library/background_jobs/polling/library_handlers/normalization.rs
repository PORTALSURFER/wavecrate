//! Normalization background-job completion handling.

use super::*;
use crate::app::controller::jobs::NormalizationResult;
use crate::app::controller::state::audio::PendingPlayback;
use std::path::Path;

impl AppController {
    /// Apply one normalization result and refresh the affected browser/waveform state.
    pub(in crate::app::controller::library::background_jobs::polling) fn handle_normalized_message(
        &mut self,
        message: NormalizationResult,
    ) {
        let source_id = message.source_id.clone();
        let relative_path = message.relative_path.clone();
        let history_key =
            crate::app::controller::history::PendingHistoryTransactionKey::Normalization {
                source_id: source_id.clone(),
                relative_path: relative_path.clone(),
            };
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .cloned();
        let playback = NormalizationPlaybackState::capture(self);

        match message.result {
            Ok((file_size, modified_ns, tag, backup)) => {
                if let Some(source) = &source {
                    let updated = self.normalized_wav_entry(
                        &source_id,
                        &relative_path,
                        file_size,
                        modified_ns,
                        tag,
                    );
                    let loaded = self.normalized_sample_is_loaded(source, &relative_path);
                    let waveform = PreservedWaveformState::capture(self);

                    self.queue_normalized_playback_if_needed(
                        source,
                        &relative_path,
                        loaded,
                        playback,
                    );
                    self.update_cached_entry(source, &relative_path, updated);
                    self.refresh_after_normalized_entry_update(
                        source,
                        &relative_path,
                        loaded,
                        waveform,
                    );
                    self.finish_normalization_success(&relative_path, history_key, backup);
                }
            }
            Err(err) => {
                self.cancel_pending_history_transaction(&history_key);
                self.set_status(format!("Normalization failed: {err}"), StatusTone::Error);
            }
        }
        self.finish_pending_file_mutation(&source_id, [relative_path.clone()]);
        self.update_normalization_progress();
    }

    fn normalized_wav_entry(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
        tag: crate::sample_sources::Rating,
    ) -> WavEntry {
        let existing = self
            .wav_index_for_path(relative_path)
            .and_then(|idx| self.wav_entries.entry(idx));
        WavEntry {
            relative_path: relative_path.to_path_buf(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped: existing.map(|entry| entry.looped).unwrap_or(false),
            sound_type: existing.and_then(|entry| entry.sound_type),
            locked: existing.map(|entry| entry.locked).unwrap_or(false),
            missing: false,
            last_played_at: existing.and_then(|entry| entry.last_played_at),
            last_curated_at: None,
            user_tag: existing.and_then(|entry| entry.user_tag.clone()),
            tag_named: false,
            normal_tags: self.normalized_entry_normal_tags(source_id, relative_path),
        }
    }

    fn normalized_entry_normal_tags(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> Vec<String> {
        self.ui_cache
            .browser
            .normal_tags
            .get(source_id)
            .and_then(|tags| tags.get(relative_path))
            .map(|tags| tags.iter().map(|tag| tag.display_label.clone()).collect())
            .unwrap_or_default()
    }

    fn normalized_sample_is_loaded(&self, source: &SampleSource, relative_path: &Path) -> bool {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == source.id && audio.relative_path == relative_path
            })
    }

    fn queue_normalized_playback_if_needed(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        loaded: bool,
        playback: NormalizationPlaybackState,
    ) {
        if !loaded || !playback.was_playing {
            return;
        }
        self.runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: source.id.clone(),
                relative_path: relative_path.to_path_buf(),
                looped: playback.was_looping,
                start_override: playback.start_override(),
                force_loaded_audio: false,
            }));
    }

    fn refresh_after_normalized_entry_update(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        loaded: bool,
        waveform: PreservedWaveformState,
    ) {
        if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
            self.mark_browser_row_metadata_projection_revision_dirty();
            self.mark_browser_search_projection_revision_dirty();
            if self.should_dispatch_browser_search_async() {
                self.dispatch_search_job();
            }
        }
        self.refresh_waveform_for_sample(source, relative_path);
        if loaded {
            waveform.restore(self);
        }
    }

    fn finish_normalization_success(
        &mut self,
        relative_path: &Path,
        history_key: crate::app::controller::history::PendingHistoryTransactionKey,
        backup: crate::app::controller::undo::OverwriteBackup,
    ) {
        self.set_status(
            format!("Normalized {}", relative_path.display()),
            StatusTone::Info,
        );
        if let Err(err) = self.finish_pending_sample_overwrite_transaction(&history_key, backup) {
            self.set_status(
                format!("Normalization undo failed: {err}"),
                StatusTone::Error,
            );
        }
    }

    fn update_normalization_progress(&mut self) {
        if !self.ui.progress.has_task(ProgressTaskKind::Normalization) {
            return;
        }
        let completed = self
            .ui
            .progress
            .task_completed(ProgressTaskKind::Normalization)
            .unwrap_or(0)
            .saturating_add(1);
        let total = self
            .ui
            .progress
            .task_total(ProgressTaskKind::Normalization)
            .unwrap_or(0);
        self.ui
            .progress
            .set_task_counts(ProgressTaskKind::Normalization, total, completed);
        if completed >= total {
            self.clear_progress_task(ProgressTaskKind::Normalization);
        }
    }
}

#[derive(Clone, Copy)]
struct NormalizationPlaybackState {
    was_playing: bool,
    position: f32,
    was_looping: bool,
}

impl NormalizationPlaybackState {
    fn capture(controller: &AppController) -> Self {
        Self {
            was_playing: controller.is_playing(),
            position: controller.ui.waveform.playhead.position,
            was_looping: controller.ui.waveform.loop_enabled,
        }
    }

    fn start_override(self) -> Option<f64> {
        self.position
            .is_finite()
            .then(|| f64::from(self.position.clamp(0.0, 1.0)))
    }
}

#[derive(Clone, Copy)]
struct PreservedWaveformState {
    view: WaveformView,
    cursor: Option<f32>,
    selection: Option<SelectionRange>,
}

impl PreservedWaveformState {
    fn capture(controller: &AppController) -> Self {
        Self {
            view: controller.ui.waveform.view,
            cursor: controller.ui.waveform.cursor,
            selection: controller.ui.waveform.selection,
        }
    }

    fn restore(self, controller: &mut AppController) {
        controller.ui.waveform.view = self.view.clamp();
        controller.ui.waveform.cursor = self.cursor;
        controller.selection_state.range.set_range(self.selection);
        controller.apply_selection(self.selection);
    }
}
