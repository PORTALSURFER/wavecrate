//! Library, browser, and file-operation background-job handlers.

mod normalization;
mod selection_export;

use super::*;
use crate::app::controller::jobs::{
    AnalysisFailuresResult, FileOpMessage, FolderScanResult, SearchResult,
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
                let completed = self
                    .ui
                    .progress
                    .task_completed(ProgressTaskKind::TrashMove)
                    .unwrap_or(0);
                self.ui
                    .progress
                    .set_task_counts(ProgressTaskKind::TrashMove, total, completed);
            }
            TrashMoveMessage::Progress { completed, detail } => {
                let total = self
                    .ui
                    .progress
                    .task_total(ProgressTaskKind::TrashMove)
                    .unwrap_or(0);
                self.ui
                    .progress
                    .set_task_counts(ProgressTaskKind::TrashMove, total, completed);
                self.ui
                    .progress
                    .set_task_detail(ProgressTaskKind::TrashMove, detail);
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
            FileOpMessage::Progress {
                completed,
                mut detail,
                item,
            } => {
                if let Some(item) = item {
                    let current_detail = self
                        .ui
                        .progress
                        .task_detail(ProgressTaskKind::FileOps)
                        .map(String::from);
                    if matches!(
                        item,
                        crate::app::controller::jobs::SampleAutoRenameProgress::Active { .. }
                    ) {
                        detail = detail.or_else(|| current_detail.clone());
                    } else if matches!(
                        item,
                        crate::app::controller::jobs::SampleAutoRenameProgress::Completed { .. }
                    ) && current_detail
                        .as_deref()
                        .is_some_and(|detail| detail.starts_with("Failed "))
                    {
                        detail = current_detail;
                    }
                    self.runtime
                        .source_lane
                        .mutations
                        .apply_auto_rename_progress(item);
                }
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
                self.clear_progress_task(ProgressTaskKind::FileOps);
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
            let (trash, neutral, keep) = if self
                .runtime
                .source_lane
                .mutations
                .source_has_pending_metadata(&message.source_id)
            {
                self.optimistic_browser_triage_partitions()
            } else {
                (message.trash, message.neutral, message.keep)
            };
            self.apply_browser_projection(message.visible, trash, neutral, keep);
            self.ui_cache.browser.search.scores = message.scores;
            self.ui.browser.search.latest_applied_search_request_id = message.request_id;
            self.ui.browser.search.search_busy = false;
            self.runtime
                .browser
                .pending_search_metadata_delta_paths
                .clear();
            self.clear_progress_task(ProgressTaskKind::Search);
            let pending_pane = self
                .runtime
                .source_lane
                .hydration
                .pending_active
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
        if self.selection_state.ctx.selected_source.as_ref() == Some(&message.source_id) {
            self.refresh_selected_source_similarity_prep_status();
        }
    }

    /// Build triage partitions from controller-owned optimistic rows while metadata writes are pending.
    fn optimistic_browser_triage_partitions(
        &mut self,
    ) -> (
        std::sync::Arc<[usize]>,
        std::sync::Arc<[usize]>,
        std::sync::Arc<[usize]>,
    ) {
        let mut trash = Vec::new();
        let mut neutral = Vec::new();
        let mut keep = Vec::new();
        for index in 0..self.wav_entries_len() {
            let Some(entry) = self.wav_entry(index) else {
                continue;
            };
            if entry.tag.is_trash() {
                trash.push(index);
            } else if entry.tag.is_keep() {
                keep.push(index);
            } else {
                neutral.push(index);
            }
        }
        (trash.into(), neutral.into(), keep.into())
    }
}
