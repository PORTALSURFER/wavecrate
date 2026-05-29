//! Selection-export background-job completion handling.

use super::*;
use crate::app::controller::jobs::{SelectionExportMessage, SelectionExportResult};

impl AppController {
    /// Apply one streamed selection-export worker message.
    pub(in crate::app::controller::library::background_jobs::polling) fn handle_selection_export_message(
        &mut self,
        message: SelectionExportMessage,
    ) {
        match message {
            SelectionExportMessage::Progress {
                request_id,
                total,
                completed,
                detail,
            } => self.handle_selection_export_progress(request_id, total, completed, detail),
            SelectionExportMessage::Finished(message) => {
                self.handle_selection_export_finished_result(message)
            }
        }
    }

    fn handle_selection_export_progress(
        &mut self,
        request_id: u64,
        total: usize,
        completed: usize,
        detail: Option<String>,
    ) {
        if self
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none_or(|pending| pending.request_id != request_id)
        {
            return;
        }
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

    fn handle_selection_export_finished_result(&mut self, message: SelectionExportResult) {
        match message {
            SelectionExportResult::Clip {
                request_id: _,
                result: Ok(success),
            } => self.apply_selection_clip_export_success(success),
            SelectionExportResult::CropNewSample {
                request_id: _,
                result: Ok(success),
            } => self.apply_selection_crop_export_success(success),
            SelectionExportResult::SliceBatch {
                request_id,
                result: Ok(success),
            } => self.handle_slice_batch_export_success(request_id, success),
            SelectionExportResult::Clip {
                request_id,
                result: Err(err),
            } => self.handle_clip_export_failure(request_id, err),
            SelectionExportResult::CropNewSample {
                request_id,
                result: Err(err),
            } => self.handle_crop_new_sample_failure(request_id, err),
            SelectionExportResult::SliceBatch {
                request_id,
                result: Err(err),
            } => self.handle_slice_batch_export_failure(request_id, err),
        }
    }

    fn handle_slice_batch_export_success(
        &mut self,
        request_id: u64,
        success: crate::app::controller::jobs::SelectionSliceBatchExportSuccess,
    ) {
        self.clear_progress_task(ProgressTaskKind::SelectionExport);
        self.runtime
            .jobs
            .clear_pending_slice_batch_export(request_id);
        self.apply_selection_slice_batch_export_success(success);
    }

    fn handle_clip_export_failure(&mut self, request_id: u64, err: String) {
        self.cancel_selection_export_history(request_id);
        if self.ui.drag.pending_external_selection_request_id == Some(request_id) {
            self.drag_drop().reset_drag();
        }
        self.record_waveform_selection_export_failure_flash();
        self.set_status(err, StatusTone::Error);
    }

    fn handle_crop_new_sample_failure(&mut self, request_id: u64, err: String) {
        self.cancel_selection_export_history(request_id);
        self.set_status(err, StatusTone::Error);
    }

    fn handle_slice_batch_export_failure(&mut self, request_id: u64, err: String) {
        self.clear_progress_task(ProgressTaskKind::SelectionExport);
        self.runtime
            .jobs
            .clear_pending_slice_batch_export(request_id);
        self.set_status(err, StatusTone::Error);
    }

    fn cancel_selection_export_history(&mut self, request_id: u64) {
        self.cancel_pending_history_transaction(
            &crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                request_id,
            },
        );
    }
}
