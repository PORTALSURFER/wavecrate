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
            let pending_pane = self
                .runtime
                .pending_active_source_hydration
                .as_ref()
                .and_then(|pending| {
                    (pending.source_id == message.source_id
                        && pending.search_request_id == Some(message.request_id))
                    .then_some(pending.pane)
                });
            if let Some(pane) = pending_pane {
                self.finish_source_loading(
                    crate::app::controller::jobs::SourceHydrationKind::ActiveSelection,
                    pane,
                );
            }
        }
    }

    /// Apply one streamed selection-export worker message.
    pub(super) fn handle_selection_export_message(
        &mut self,
        message: crate::app::controller::jobs::SelectionExportMessage,
    ) {
        match message {
            crate::app::controller::jobs::SelectionExportMessage::Progress {
                request_id,
                total,
                completed,
                detail,
            } => {
                if self
                    .runtime
                    .jobs
                    .pending_slice_batch_export()
                    .is_some_and(|pending| pending.request_id == request_id)
                {
                    progress::ensure_progress_visible(
                        self,
                        ProgressTaskKind::SelectionExport,
                        "Saving slices",
                        total,
                        false,
                    );
                    progress::update_progress_totals(
                        self,
                        ProgressTaskKind::SelectionExport,
                        total,
                        completed,
                        detail,
                    );
                }
            }
            crate::app::controller::jobs::SelectionExportMessage::Finished(message) => {
                self.handle_selection_export_finished_result(message)
            }
        }
    }

    fn handle_selection_export_finished_result(
        &mut self,
        message: crate::app::controller::jobs::SelectionExportResult,
    ) {
        match message {
            crate::app::controller::jobs::SelectionExportResult::Clip {
                request_id,
                result: Ok(success),
            } => {
                let _ = request_id;
                self.apply_selection_clip_export_success(success);
            }
            crate::app::controller::jobs::SelectionExportResult::CropNewSample {
                request_id,
                result: Ok(success),
            } => {
                let _ = request_id;
                self.apply_selection_crop_export_success(success);
            }
            crate::app::controller::jobs::SelectionExportResult::SliceBatch {
                request_id,
                result: Ok(success),
            } => {
                if self.ui.progress.task == Some(ProgressTaskKind::SelectionExport) {
                    self.clear_progress();
                }
                self.runtime
                    .jobs
                    .clear_pending_slice_batch_export(request_id);
                self.apply_selection_slice_batch_export_success(success);
            }
            crate::app::controller::jobs::SelectionExportResult::Clip {
                request_id,
                result: Err(err),
            } => {
                self.cancel_pending_history_transaction(
                    &crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                        request_id,
                    },
                );
                if self.ui.drag.pending_external_selection_request_id == Some(request_id) {
                    self.drag_drop().reset_drag();
                }
                self.record_waveform_selection_export_failure_flash();
                self.set_status(err, StatusTone::Error);
            }
            crate::app::controller::jobs::SelectionExportResult::CropNewSample {
                request_id,
                result: Err(err),
            } => {
                self.cancel_pending_history_transaction(
                    &crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                        request_id,
                    },
                );
                self.set_status(err, StatusTone::Error);
            }
            crate::app::controller::jobs::SelectionExportResult::SliceBatch {
                request_id,
                result: Err(err),
            } => {
                if self.ui.progress.task == Some(ProgressTaskKind::SelectionExport) {
                    self.clear_progress();
                }
                self.runtime
                    .jobs
                    .clear_pending_slice_batch_export(request_id);
                self.set_status(err, StatusTone::Error);
            }
        }
    }

    /// Apply one normalization result and refresh the affected browser/waveform state.
    pub(super) fn handle_normalized_message(&mut self, message: NormalizationResult) {
        let history_key =
            crate::app::controller::history::PendingHistoryTransactionKey::Normalization {
                source_id: message.source_id.clone(),
                relative_path: message.relative_path.clone(),
            };
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
            Ok((file_size, modified_ns, tag, backup)) => {
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
                    let preserved_view = self.ui.waveform.view;
                    let preserved_cursor = self.ui.waveform.cursor;
                    let preserved_selection = self.ui.waveform.selection;

                    if is_currently_loaded && was_playing {
                        let start_override = if playhead_position.is_finite() {
                            Some(f64::from(playhead_position.clamp(0.0, 1.0)))
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
                                force_loaded_audio: false,
                            }));
                    }

                    self.update_cached_entry(source, &message.relative_path, updated);

                    if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
                        self.mark_browser_row_metadata_projection_revision_dirty();
                        self.mark_browser_search_projection_revision_dirty();
                        if self.should_dispatch_browser_search_async() {
                            self.dispatch_search_job();
                        }
                    }
                    self.refresh_waveform_for_sample(source, &message.relative_path);
                    if is_currently_loaded {
                        self.ui.waveform.view = preserved_view.clamp();
                        self.ui.waveform.cursor = preserved_cursor;
                        self.selection_state.range.set_range(preserved_selection);
                        self.apply_selection(preserved_selection);
                    }

                    self.set_status(
                        format!("Normalized {}", message.relative_path.display()),
                        StatusTone::Info,
                    );
                    if let Err(err) =
                        self.finish_pending_sample_overwrite_transaction(&history_key, backup)
                    {
                        self.set_status(
                            format!("Normalization undo failed: {err}"),
                            StatusTone::Error,
                        );
                    }
                }
            }
            Err(err) => {
                self.cancel_pending_history_transaction(&history_key);
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
