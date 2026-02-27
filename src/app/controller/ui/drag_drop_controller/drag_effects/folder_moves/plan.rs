use super::super::super::DragDropController;
use super::worker::{run_folder_move_task, run_folder_sample_move_task};
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{
    FileOpMessage, FileOpResult, FolderMoveRequest, FolderSampleMoveRequest,
};
use crate::app::state::{DragSample, ProgressTaskKind};
use crate::sample_sources::SourceId;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use tracing::{info, warn};

impl DragDropController<'_> {
    /// Enqueue a background move for a single sample dropped into a folder.
    pub(crate) fn handle_sample_drop_to_folder(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        target_folder: &Path,
    ) {
        let sample = DragSample {
            source_id,
            relative_path,
        };
        self.handle_samples_drop_to_folder(std::slice::from_ref(&sample), target_folder);
    }

    /// Enqueue background moves for multiple samples dropped into a folder.
    pub(crate) fn handle_samples_drop_to_folder(
        &mut self,
        samples: &[DragSample],
        target_folder: &Path,
    ) {
        if samples.is_empty() {
            return;
        }
        info!(
            "Folder drop requested: sample_count={} target={}",
            samples.len(),
            target_folder.display()
        );
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return;
        }
        let source_id = samples[0].source_id.clone();
        if samples.iter().any(|sample| sample.source_id != source_id) {
            self.set_status(
                "Samples must come from the same source to move into a folder",
                StatusTone::Warning,
            );
            return;
        }
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .cloned()
        else {
            warn!("Folder move: missing source {:?}", source_id);
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        if self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|selected| selected != &source.id)
        {
            warn!(
                "Folder move blocked: selected source {:?} differs from sample source {:?}",
                self.selection_state.ctx.selected_source, source.id
            );
            self.set_status(
                "Switch to the sample's source before moving into its folders",
                StatusTone::Warning,
            );
            return;
        }
        let mut requests = Vec::new();
        let mut errors = Vec::new();
        let mut skipped = 0usize;
        for sample in samples {
            let file_name = match sample.relative_path.file_name() {
                Some(name) => name.to_owned(),
                None => {
                    errors.push(format!(
                        "Sample name unavailable for move: {}",
                        sample.relative_path.display()
                    ));
                    continue;
                }
            };
            let new_relative = if target_folder.as_os_str().is_empty() {
                PathBuf::from(file_name)
            } else {
                target_folder.join(file_name)
            };
            if new_relative == sample.relative_path {
                skipped += 1;
                continue;
            }
            requests.push(FolderSampleMoveRequest {
                relative_path: sample.relative_path.clone(),
                target_relative: new_relative,
            });
        }
        if requests.is_empty() {
            if let Some(err) = errors.first() {
                self.set_status(err.clone(), StatusTone::Error);
            } else if skipped > 0 {
                self.set_status("Samples are already in that folder", StatusTone::Info);
            }
            return;
        }
        let label = if requests.len() == 1 {
            "Moving sample"
        } else {
            "Moving samples"
        };
        self.set_status(format!("{label}..."), StatusTone::Busy);
        self.show_status_progress(
            ProgressTaskKind::FileOps,
            label.to_string(),
            requests.len(),
            true,
        );
        let cancel = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        {
            let result = run_folder_sample_move_task(
                source.id.clone(),
                source.root.clone(),
                requests,
                errors,
                cancel,
                None,
            );
            let message = FileOpMessage::Finished(FileOpResult::FolderSampleMove(result));
            if let FileOpMessage::Finished(FileOpResult::FolderSampleMove(result)) = message {
                self.apply_folder_sample_move_result(result);
            }
            if self.ui.progress.task == Some(ProgressTaskKind::FileOps) {
                self.clear_progress();
            }
        }
        #[cfg(not(test))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.runtime.jobs.start_file_ops(rx, cancel.clone());
            std::thread::spawn(move || {
                let result = run_folder_sample_move_task(
                    source.id.clone(),
                    source.root.clone(),
                    requests,
                    errors,
                    cancel,
                    Some(&tx),
                );
                let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderSampleMove(
                    result,
                )));
            });
        }
    }

    /// Enqueue a background move for a folder dropped onto another folder.
    pub(crate) fn handle_folder_drop_to_folder(
        &mut self,
        source_id: SourceId,
        folder: PathBuf,
        target_folder: &Path,
    ) {
        info!(
            "Folder drag requested: source_id={:?} folder={} target={}",
            source_id,
            folder.display(),
            target_folder.display()
        );
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return;
        }
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .cloned()
        else {
            warn!("Folder drag: missing source {:?}", source_id);
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        if folder.as_os_str().is_empty() {
            self.set_status("Root folder cannot be moved", StatusTone::Warning);
            return;
        }
        if target_folder == folder {
            self.set_status("Folder is already there", StatusTone::Info);
            return;
        }
        if target_folder.starts_with(&folder) {
            self.set_status("Cannot move a folder into itself", StatusTone::Warning);
            return;
        }
        if self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|selected| selected != &source.id)
        {
            warn!(
                "Folder drag blocked: selected source {:?} differs from folder source {:?}",
                self.selection_state.ctx.selected_source, source.id
            );
            self.set_status(
                "Switch to the folder's source before moving it",
                StatusTone::Warning,
            );
            return;
        }
        let label = "Moving folder";
        self.set_status(format!("{label}..."), StatusTone::Busy);
        self.show_status_progress(ProgressTaskKind::FileOps, label.to_string(), 1, true);
        let cancel = Arc::new(AtomicBool::new(false));
        let request = FolderMoveRequest {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            folder,
            target_folder: target_folder.to_path_buf(),
        };
        #[cfg(test)]
        {
            let result = run_folder_move_task(request, cancel, None);
            let message = FileOpMessage::Finished(FileOpResult::FolderMove(result));
            if let FileOpMessage::Finished(FileOpResult::FolderMove(result)) = message {
                self.apply_folder_move_result(result);
            }
            if self.ui.progress.task == Some(ProgressTaskKind::FileOps) {
                self.clear_progress();
            }
        }
        #[cfg(not(test))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.runtime.jobs.start_file_ops(rx, cancel.clone());
            std::thread::spawn(move || {
                let result = run_folder_move_task(request, cancel, Some(&tx));
                let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderMove(result)));
            });
        }
    }
}
