//! Drop-target drag/drop handling for cross-source copies and moves.
//!
//! This module keeps controller-side planning and result application lightweight while the
//! filesystem and source-database work runs on the existing file-op worker pipeline.

use super::super::DragDropController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferMetadata, DropTargetTransferRequest, FileOpMessage,
    FileOpResult,
};
use crate::app::state::{DragSample, ProgressTaskKind};
use crate::sample_sources::{Rating, SourceId};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use tracing::info;

mod apply_result;
mod transactions;
mod worker;

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

    /// Resolve drag samples into worker requests and collect preflight validation errors.
    fn collect_drop_target_transfer_requests(
        &mut self,
        samples: &[DragSample],
    ) -> (Vec<DropTargetTransferRequest>, Vec<String>) {
        let mut requests = Vec::new();
        let mut errors = Vec::new();
        for sample in samples {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == sample.source_id)
                .cloned()
            else {
                errors.push(format!(
                    "Source not available for drop: {}",
                    sample.relative_path.display()
                ));
                continue;
            };
            if sample.relative_path.file_name().is_none() {
                errors.push("Sample name unavailable for drop".to_string());
                continue;
            }
            let absolute = source.root.join(&sample.relative_path);
            if !absolute.exists() {
                errors.push(format!("File missing: {}", sample.relative_path.display()));
                continue;
            }
            let metadata = self.cached_drop_target_metadata(&source, &sample.relative_path);
            requests.push(DropTargetTransferRequest {
                source_id: source.id,
                source_root: source.root,
                relative_path: sample.relative_path.clone(),
                metadata,
            });
        }
        (requests, errors)
    }

    /// Read cache-backed metadata for a dragged sample without falling back to source DB I/O.
    fn cached_drop_target_metadata(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        relative_path: &Path,
    ) -> Option<DropTargetTransferMetadata> {
        if let Some(cache) = self.cache.wav.entries.get(&source.id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(entry) = cache.entry(index)
        {
            return Some(DropTargetTransferMetadata {
                tag: entry.tag,
                looped: entry.looped,
                locked: entry.locked,
                last_played_at: entry.last_played_at,
            });
        }
        if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
            && let Some(index) = self.wav_entries.lookup.get(relative_path).copied()
            && let Some(entry) = self.wav_entries.entry(index)
        {
            return Some(DropTargetTransferMetadata {
                tag: entry.tag,
                looped: entry.looped,
                locked: entry.locked,
                last_played_at: entry.last_played_at,
            });
        }
        None
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

/// Build the destination-relative path for a drop-target move.
fn move_destination_relative(
    target_root: &Path,
    target_folder: &Path,
    file_name: &OsStr,
) -> Result<PathBuf, String> {
    let relative = if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    };
    let destination = target_root.join(&relative);
    if destination.exists() {
        return Err(format!(
            "A file already exists at {}",
            destination.display()
        ));
    }
    Ok(relative)
}

/// Build the destination-relative path for a drop-target copy, adding a suffix on collision.
fn copy_destination_relative(
    target_root: &Path,
    target_folder: &Path,
    file_name: &OsStr,
) -> Result<PathBuf, String> {
    let base = if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    };
    if !target_root.join(&base).exists() {
        return Ok(base);
    }
    let stem = Path::new(file_name)
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".to_string());
    let extension = Path::new(file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    for index in 1..=999 {
        let suffix = format!("{stem}_copy{index:03}");
        let file_name = if let Some(ext) = &extension {
            format!("{suffix}.{ext}")
        } else {
            suffix
        };
        let candidate = if target_folder.as_os_str().is_empty() {
            PathBuf::from(&file_name)
        } else {
            target_folder.join(&file_name)
        };
        if !target_root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Failed to find destination file name".into())
}

/// Format the progress title for a drop-target transfer batch.
fn progress_title(kind: DropTargetTransferKind, count: usize) -> &'static str {
    match (kind, count) {
        (DropTargetTransferKind::Copy, 1) => "Copying sample",
        (DropTargetTransferKind::Copy, _) => "Copying samples",
        (DropTargetTransferKind::Move, 1) => "Moving sample",
        (DropTargetTransferKind::Move, _) => "Moving samples",
    }
}
