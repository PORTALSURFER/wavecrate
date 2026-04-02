//! Drop-target drag/drop handling for cross-source copies and moves.
//!
//! This module keeps controller-side planning and result application lightweight while the
//! filesystem and source-database work runs on the existing file-op worker pipeline.

use super::super::DragDropController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferRequest, FileOpMessage, FileOpResult,
};
use crate::app::state::{DragSample, ProgressTaskKind};
use crate::sample_sources::{Rating, SourceId};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use tracing::info;

mod apply_result;
mod paths;
mod planning;
mod transactions;
mod worker;

use paths::progress_title;
use worker::run_drop_target_transfer_task;

/// Metadata copied from the source DB row onto the copied or moved target entry.
#[derive(Clone, Copy, Debug)]
pub(super) struct DroppedSampleMetadata {
    tag: Rating,
    looped: bool,
    locked: bool,
    last_played_at: Option<i64>,
}

impl DragDropController<'_> {
    /// Handle a single dragged sample dropped onto a configured drop target.
    pub(crate) fn handle_sample_drop_to_drop_target(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        target_path: PathBuf,
        copy_requested: bool,
    ) {
        let sample = DragSample {
            source_id,
            relative_path,
        };
        self.handle_samples_drop_to_drop_target(
            std::slice::from_ref(&sample),
            target_path,
            copy_requested,
        );
    }

    /// Handle multiple dragged samples dropped onto a configured drop target.
    pub(crate) fn handle_samples_drop_to_drop_target(
        &mut self,
        samples: &[DragSample],
        target_path: PathBuf,
        copy_requested: bool,
    ) {
        if samples.is_empty() {
            return;
        }
        info!(
            "Drop target requested: sample_count={} target={}",
            samples.len(),
            target_path.display()
        );
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return;
        }
        let Some(target) = self.resolve_drop_target_location(&target_path) else {
            self.set_status(
                "Drop target is not inside a configured source",
                StatusTone::Warning,
            );
            return;
        };
        let target_dir = if target.relative_folder.as_os_str().is_empty() {
            target.source.root.clone()
        } else {
            target.source.root.join(&target.relative_folder)
        };
        if !target_dir.is_dir() {
            self.set_status(
                format!("Drop target missing: {}", target_dir.display()),
                StatusTone::Error,
            );
            return;
        }
        if !copy_requested
            && samples
                .iter()
                .all(|sample| sample.source_id == target.source.id)
        {
            self.handle_samples_drop_to_folder(samples, &target.relative_folder);
            return;
        }
        let kind = if copy_requested {
            DropTargetTransferKind::Copy
        } else {
            DropTargetTransferKind::Move
        };
        let (requests, errors) = self.collect_drop_target_transfer_requests(samples);
        if requests.is_empty() {
            if let Some(err) = errors.first() {
                self.set_status(err.clone(), StatusTone::Error);
            }
            return;
        }
        let progress_title = progress_title(kind, requests.len());
        self.set_status(format!("{progress_title}..."), StatusTone::Busy);
        self.show_status_progress(
            ProgressTaskKind::FileOps,
            progress_title.to_string(),
            requests.len(),
            true,
        );
        self.spawn_drop_target_transfer_job(
            kind,
            target.source.id.clone(),
            target.source.root.clone(),
            target.relative_folder.clone(),
            requests,
            errors,
        );
    }

    /// Transfer dragged samples into one explicit source folder, optionally copying instead of moving.
    pub(crate) fn handle_samples_transfer_to_source_folder(
        &mut self,
        samples: &[DragSample],
        target_source_id: SourceId,
        target_relative_folder: PathBuf,
        copy_requested: bool,
    ) {
        if samples.is_empty() {
            return;
        }
        let Some(target_source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == target_source_id)
            .cloned()
        else {
            self.set_status("Target source not available for drop", StatusTone::Error);
            return;
        };
        let target_dir = if target_relative_folder.as_os_str().is_empty() {
            target_source.root.clone()
        } else {
            target_source.root.join(&target_relative_folder)
        };
        if !target_dir.is_dir() {
            self.set_status(
                format!("Folder not found: {}", target_relative_folder.display()),
                StatusTone::Error,
            );
            return;
        }
        if !copy_requested
            && samples
                .iter()
                .all(|sample| sample.source_id == target_source.id)
        {
            self.handle_samples_drop_to_folder(samples, &target_relative_folder);
            return;
        }
        let kind = if copy_requested {
            DropTargetTransferKind::Copy
        } else {
            DropTargetTransferKind::Move
        };
        let (requests, errors) = self.collect_drop_target_transfer_requests(samples);
        if requests.is_empty() {
            if let Some(err) = errors.first() {
                self.set_status(err.clone(), StatusTone::Error);
            }
            return;
        }
        let progress_title = progress_title(kind, requests.len());
        self.set_status(format!("{progress_title}..."), StatusTone::Busy);
        self.show_status_progress(
            ProgressTaskKind::FileOps,
            progress_title.to_string(),
            requests.len(),
            true,
        );
        self.spawn_drop_target_transfer_job(
            kind,
            target_source.id,
            target_source.root,
            target_relative_folder,
            requests,
            errors,
        );
    }

    /// Start the drop-target worker or run it inline during tests.
    fn spawn_drop_target_transfer_job(
        &mut self,
        kind: DropTargetTransferKind,
        target_source_id: SourceId,
        target_root: PathBuf,
        target_relative_folder: PathBuf,
        requests: Vec<DropTargetTransferRequest>,
        errors: Vec<String>,
    ) {
        let cancel = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        {
            let result = run_drop_target_transfer_task(
                kind,
                target_source_id,
                target_root,
                target_relative_folder,
                requests,
                errors,
                cancel,
                None,
            );
            self.finish_drop_target_transfer_job(result);
        }
        #[cfg(not(test))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.runtime.jobs.start_file_ops(rx, cancel.clone());
            std::thread::spawn(move || {
                let result = run_drop_target_transfer_task(
                    kind,
                    target_source_id,
                    target_root,
                    target_relative_folder,
                    requests,
                    errors,
                    cancel,
                    Some(&tx),
                );
                let _ = tx.send(FileOpMessage::Finished(FileOpResult::DropTargetTransfer(
                    result,
                )));
            });
        }
    }

    #[cfg(test)]
    /// Apply one inline test result and clear the progress overlay.
    fn finish_drop_target_transfer_job(
        &mut self,
        result: crate::app::controller::jobs::DropTargetTransferResult,
    ) {
        let message = FileOpMessage::Finished(FileOpResult::DropTargetTransfer(result));
        if let FileOpMessage::Finished(FileOpResult::DropTargetTransfer(result)) = message {
            self.apply_drop_target_transfer_result(result);
        }
        if self.ui.progress.task == Some(ProgressTaskKind::FileOps) {
            self.clear_progress();
        }
    }
}
