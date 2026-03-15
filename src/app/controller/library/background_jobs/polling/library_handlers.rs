//! Library, browser, and file-operation background-job handlers.

use super::*;
use crate::app::controller::jobs::{
    AnalysisFailuresResult, FileOpMessage, FolderScanResult, NormalizationResult, SearchResult,
};

impl AppController {
    /// Apply one scan worker message.
    pub(super) fn handle_scan_message(&mut self, message: ScanJobMessage) {
        match message {
            ScanJobMessage::Progress { completed, detail } => {
                scan::handle_scan_progress(self, completed, detail);
            }
            ScanJobMessage::Finished(result) => {
                scan::handle_scan_finished(self, result);
            }
        }
    }

    /// Apply one folder-scan result when it still matches the latest request.
    pub(super) fn handle_folder_scan_finished_message(&mut self, message: FolderScanResult) {
        if !self
            .runtime
            .jobs
            .folder_scan_matches(message.request_id, &message.source_id)
        {
            return;
        }
        self.runtime.jobs.clear_folder_scan();
        self.apply_folder_scan_result(message);
    }

    /// Apply one trash-move progress or completion message.
    pub(super) fn handle_trash_move_message(&mut self, message: TrashMoveMessage) {
        match message {
            TrashMoveMessage::SetTotal(total) => {
                self.ui
                    .progress
                    .set_counts(total, self.ui.progress.completed);
            }
            TrashMoveMessage::Progress { completed, detail } => {
                self.ui
                    .progress
                    .set_counts(self.ui.progress.total, completed);
                self.ui.progress.set_detail(detail);
            }
            TrashMoveMessage::Finished(result) => {
                self.runtime.jobs.clear_trash_move();
                self.apply_trash_move_finished(result);
            }
        }
    }

    /// Apply one file-operation progress or completion message.
    pub(super) fn handle_file_ops_message(&mut self, message: FileOpMessage) {
        match message {
            FileOpMessage::Progress { completed, detail } => {
                progress::update_progress_detail(
                    self,
                    ProgressTaskKind::FileOps,
                    completed,
                    detail,
                );
            }
            FileOpMessage::Finished(result) => {
                self.runtime.jobs.clear_file_ops();
                self.apply_file_op_result(result);
                if self.ui.progress.task == Some(ProgressTaskKind::FileOps) {
                    self.clear_progress();
                }
            }
        }
    }

    /// Apply one analysis-failures load result into the browser cache.
    pub(super) fn handle_analysis_failures_loaded_message(
        &mut self,
        message: AnalysisFailuresResult,
    ) {
        self.ui_cache
            .browser
            .analysis_failures_pending
            .remove(&message.source_id);
        match message.result {
            Ok(failures) => {
                if failures.is_empty() {
                    self.ui_cache
                        .browser
                        .analysis_failures
                        .remove(&message.source_id);
                } else {
                    self.ui_cache
                        .browser
                        .analysis_failures
                        .insert(message.source_id, failures);
                }
            }
            Err(err) => {
                self.ui_cache
                    .browser
                    .analysis_failures
                    .remove(&message.source_id);
                self.set_status(
                    format!("Failed to load analysis failures: {err}"),
                    StatusTone::Warning,
                );
            }
        }
    }

    /// Apply one browser-search result when it still matches the active request.
    pub(super) fn handle_browser_search_finished_message(&mut self, message: SearchResult) {
        if Some(&message.source_id) == self.selection_state.ctx.selected_source.as_ref()
            && message.query == self.ui.browser.search.search_query
            && message.request_id == self.ui.browser.search.latest_search_request_id
        {
            self.mark_browser_search_projection_revision_dirty();
            self.apply_browser_projection(
                message.visible,
                message.trash,
                message.neutral,
                message.keep,
            );
            self.ui_cache.browser.search.scores = message.scores;
            self.ui.browser.search.latest_applied_search_request_id = message.request_id;
            self.ui.browser.search.search_busy = false;
        }
    }

    /// Apply one normalization result and refresh the affected browser/waveform state.
    pub(super) fn handle_normalized_message(&mut self, message: NormalizationResult) {
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == message.source_id)
            .cloned();
        let was_playing = self.is_playing();
        let playhead_position = self.ui.waveform.playhead.position;
        let was_looping = self.ui.waveform.loop_enabled;

        match message.result {
            Ok((file_size, modified_ns, tag)) => {
                if let Some(source) = &source {
                    let updated = WavEntry {
                        relative_path: message.relative_path.clone(),
                        file_size,
                        modified_ns,
                        content_hash: None,
                        tag,
                        looped: self
                            .wav_index_for_path(&message.relative_path)
                            .and_then(|idx| self.wav_entries.entry(idx))
                            .map(|e| e.looped)
                            .unwrap_or(false),
                        locked: self
                            .wav_index_for_path(&message.relative_path)
                            .and_then(|idx| self.wav_entries.entry(idx))
                            .map(|e| e.locked)
                            .unwrap_or(false),
                        missing: false,
                        last_played_at: self
                            .wav_index_for_path(&message.relative_path)
                            .and_then(|idx| self.wav_entries.entry(idx))
                            .and_then(|e| e.last_played_at),
                    };

                    let is_currently_loaded = self
                        .sample_view
                        .wav
                        .loaded_audio
                        .as_ref()
                        .is_some_and(|audio| {
                            audio.source_id == source.id
                                && audio.relative_path == message.relative_path
                        });

                    if is_currently_loaded && was_playing {
                        let start_override = if playhead_position.is_finite() {
                            Some(playhead_position.clamp(0.0, 1.0))
                        } else {
                            None
                        };
                        self.runtime
                            .jobs
                            .set_pending_playback(Some(PendingPlayback {
                                source_id: source.id.clone(),
                                relative_path: message.relative_path.clone(),
                                looped: was_looping,
                                start_override,
                            }));
                    }

                    self.update_cached_entry(source, &message.relative_path, updated);

                    if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
                        self.rebuild_browser_lists();
                    }
                    self.refresh_waveform_for_sample(source, &message.relative_path);

                    self.set_status(
                        format!("Normalized {}", message.relative_path.display()),
                        StatusTone::Info,
                    );
                }
            }
            Err(err) => {
                self.set_status(format!("Normalization failed: {err}"), StatusTone::Error);
            }
        }
        if self.ui.progress.task == Some(ProgressTaskKind::Normalization) {
            self.ui.progress.completed = self.ui.progress.completed.saturating_add(1);
            if self.ui.progress.completed >= self.ui.progress.total {
                self.clear_progress();
            }
        }
    }
}
