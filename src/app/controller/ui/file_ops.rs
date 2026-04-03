//! Apply background file operation results to controller state.

use super::*;
use crate::app::controller::jobs::{
    ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage, FileOpResult, FolderCreateResult,
    FolderDeleteResult, FolderRenameResult, SampleDeleteResult, SampleRenameResult,
    SelectionEditCommitResult, UndoFileOpResult, UndoFileOutcome, WaveformSlideCommitResult,
};
use crate::app::controller::undo::{DeferredUndo, UndoDirection};
use crate::app::controller::undo_jobs;
use std::sync::{Arc, atomic::AtomicBool};
use tracing::warn;

impl AppController {
    /// Apply a completed background file operation to controller state.
    pub(crate) fn apply_file_op_result(&mut self, result: FileOpResult) {
        match result {
            FileOpResult::ClipboardPaste(result) => self.apply_clipboard_paste_result(result),
            FileOpResult::RetainedDeleteResolution(result) => {
                self.apply_retained_delete_resolution_result(result);
            }
            FileOpResult::DropTargetTransfer(result) => {
                self.drag_drop().apply_drop_target_transfer_result(result);
            }
            FileOpResult::SourceMove(result) => {
                self.drag_drop().apply_source_move_result(result);
            }
            FileOpResult::FolderSampleMove(result) => {
                self.drag_drop().apply_folder_sample_move_result(result);
            }
            FileOpResult::FolderMove(result) => {
                self.drag_drop().apply_folder_move_result(result);
            }
            FileOpResult::SampleDelete(result) => self.apply_sample_delete_result(result),
            FileOpResult::SampleRename(result) => self.apply_sample_rename_result(result),
            FileOpResult::FolderCreate(result) => self.apply_folder_create_result(result),
            FileOpResult::FolderRename(result) => self.apply_folder_rename_result(result),
            FileOpResult::FolderDelete(result) => self.apply_folder_delete_result(result),
            FileOpResult::SelectionEditCommit(result) => {
                self.apply_selection_edit_commit_result(result);
            }
            FileOpResult::WaveformSlideCommit(result) => {
                self.apply_waveform_slide_commit_result(result);
            }
            FileOpResult::UndoFile(result) => self.apply_undo_file_result(result),
        }
    }

    fn apply_clipboard_paste_result(&mut self, result: ClipboardPasteResult) {
        match &result.outcome {
            ClipboardPasteOutcome::Source { source_id, added } => {
                if let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .cloned()
                {
                    if let Some(last) = added.last() {
                        self.runtime
                            .jobs
                            .set_pending_select_path(Some(last.relative_path.clone()));
                    }
                    if !added.is_empty() {
                        self.invalidate_wav_entries_for_source(&source);
                    }
                    for sample in added {
                        self.enqueue_similarity_for_new_sample(
                            &source,
                            &sample.relative_path,
                            sample.file_size,
                            sample.modified_ns,
                        );
                    }
                } else {
                    self.set_status("Source not available for paste", StatusTone::Error);
                    return;
                }
            }
        }
        self.report_clipboard_paste_summary(&result);
    }

    fn report_clipboard_paste_summary(&mut self, result: &ClipboardPasteResult) {
        let added = match &result.outcome {
            ClipboardPasteOutcome::Source { added, .. } => added.len(),
        };
        if added == 0 && result.errors.is_empty() && result.skipped == 0 {
            if result.cancelled {
                self.set_status("File operation cancelled", StatusTone::Warning);
            } else {
                self.set_status("No files processed", StatusTone::Warning);
            }
            return;
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = format!(
            "{} {} file(s) into {}",
            result.action_past_tense, added, result.target_label
        );
        if result.skipped > 0 {
            message.push_str(&format!(" (skipped {})", result.skipped));
        }
        if !result.errors.is_empty() {
            message.push_str(&format!(" with {} error(s)", result.errors.len()));
        }
        if result.cancelled {
            message.push_str(" (cancelled)");
        }
        self.set_status(message, tone);
        for err in &result.errors {
            warn!(
                error = %err,
                action = %result.action_past_tense,
                target = %result.target_label,
                skipped = result.skipped,
                cancelled = result.cancelled,
                "File operation error"
            );
        }
    }

    fn apply_undo_file_result(&mut self, result: UndoFileOpResult) {
        let Some(pending) = self.history.pending_undo.take() else {
            self.set_status(
                "Undo completion arrived without a pending entry",
                StatusTone::Error,
            );
            return;
        };
        let action_label = match pending.direction {
            UndoDirection::Undo => "Undo",
            UndoDirection::Redo => "Redo",
        };
        if result.cancelled {
            self.restore_deferred_entry(pending);
            self.set_status(format!("{action_label} cancelled"), StatusTone::Warning);
            return;
        }
        match result.result {
            Ok(outcome) => {
                self.apply_undo_file_outcome(&outcome);
                self.commit_deferred_entry(pending);
            }
            Err(err) => {
                self.restore_deferred_entry(pending);
                self.set_status(format!("{action_label} failed: {err}"), StatusTone::Error);
            }
        }
    }

    /// Start a deferred undo/redo job and track its completion.
    pub(crate) fn begin_deferred_undo_job(&mut self, pending: DeferredUndo<AppController>) {
        let label = pending.entry.label.clone();
        let direction = pending.direction;
        let job = pending.job.clone();
        let title = match direction {
            UndoDirection::Undo => format!("Undoing {label}"),
            UndoDirection::Redo => format!("Redoing {label}"),
        };
        self.history.pending_undo = Some(pending);
        self.set_status(format!("{title}..."), StatusTone::Busy);
        self.show_status_progress(crate::app::state::ProgressTaskKind::FileOps, title, 1, true);
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = undo_jobs::run_undo_file_job(job, cancel, Some(&tx));
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::UndoFile(result)));
        });
    }

    fn apply_undo_file_outcome(&mut self, outcome: &UndoFileOutcome) {
        match outcome {
            UndoFileOutcome::Overwrite {
                source_id,
                relative_path,
                file_size,
                modified_ns,
                tag,
                looped,
                last_played_at,
            } => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for undo", StatusTone::Error);
                    return;
                };
                let entry = WavEntry {
                    relative_path: relative_path.clone(),
                    file_size: *file_size,
                    modified_ns: *modified_ns,
                    content_hash: None,
                    tag: *tag,
                    looped: *looped,
                    locked: false,
                    missing: false,
                    last_played_at: *last_played_at,
                };
                self.update_cached_entry(&source, relative_path, entry);
                self.refresh_waveform_for_sample(&source, relative_path);
            }
            UndoFileOutcome::Removed {
                source_id,
                relative_path,
            } => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for undo", StatusTone::Error);
                    return;
                };
                self.prune_cached_sample(&source, relative_path);
            }
            UndoFileOutcome::Restored {
                source_id,
                relative_path,
                file_size,
                modified_ns,
                tag,
                looped,
                last_played_at,
            } => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| &source.id == source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for undo", StatusTone::Error);
                    return;
                };
                self.insert_cached_entry(
                    &source,
                    WavEntry {
                        relative_path: relative_path.clone(),
                        file_size: *file_size,
                        modified_ns: *modified_ns,
                        content_hash: None,
                        tag: *tag,
                        looped: *looped,
                        locked: false,
                        missing: false,
                        last_played_at: *last_played_at,
                    },
                );
                self.refresh_waveform_for_sample(&source, relative_path);
            }
        }
    }

    fn commit_deferred_entry(&mut self, pending: DeferredUndo<AppController>) {
        match pending.direction {
            UndoDirection::Undo => {
                let label = pending.entry.label.clone();
                pending.entry.run_post_undo(self);
                self.history.undo_stack.push_redo_entry(pending.entry);
                self.set_status(format!("Undid {label}"), StatusTone::Info);
            }
            UndoDirection::Redo => {
                let label = pending.entry.label.clone();
                pending.entry.run_post_redo(self);
                self.history.undo_stack.restore_undo_entry(pending.entry);
                self.set_status(format!("Redid {label}"), StatusTone::Info);
            }
        }
    }

    fn apply_sample_delete_result(&mut self, result: SampleDeleteResult) {
        self.finish_pending_file_mutation(&result.source_id, result.requested_paths.clone());
        let selected_source_id = self.selected_source_id();
        let similar_query = self.ui.browser.search.similar_query.clone();
        for path in &result.deleted_paths {
            if let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == result.source_id)
                .cloned()
            {
                self.prune_cached_sample(&source, path);
            }
        }
        if !result.deleted_paths.is_empty() {
            crate::app::controller::library::wavs::schedule_similarity_filter_rebuild_after_delete_with_state(
                self,
                selected_source_id,
                similar_query,
                &result.deleted_paths.iter().cloned().collect::<std::collections::HashSet<_>>(),
            );
            crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
            self.browser().restore_browser_focus_after_delete(result.next_focus);
            self.set_status(
                format!("Deleted {} sample(s)", result.deleted_paths.len()),
                StatusTone::Info,
            );
        }
        if let Some(err) = result.last_error {
            self.set_status(format!("Delete failed: {err}"), StatusTone::Error);
        }
    }

    fn apply_sample_rename_result(&mut self, result: SampleRenameResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.old_relative.clone()]);
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for rename", StatusTone::Error);
                    return;
                };
                if let Some(entry) = result.entry {
                    self.update_cached_entry(&source, &result.old_relative, entry);
                }
                if result.resume_playback {
                    self.runtime.jobs.set_pending_playback(Some(PendingPlayback {
                        source_id: result.source_id.clone(),
                        relative_path: result.new_relative.clone(),
                        looped: result.resume_looped,
                        start_override: result.resume_start_override,
                        force_loaded_audio: false,
                    }));
                }
                self.refresh_waveform_for_sample(&source, &result.new_relative);
                self.set_status(
                    format!(
                        "Renamed {} to {}",
                        result.old_relative.display(),
                        result.new_relative.display()
                    ),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn apply_folder_create_result(&mut self, result: FolderCreateResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                self.update_manual_folders(|set| {
                    set.insert(result.relative_path.clone());
                });
                self.update_disk_folders(|set| {
                    set.insert(result.relative_path.clone());
                });
                self.refresh_folder_browser();
                self.focus_folder_by_path(&result.relative_path);
                self.set_status(
                    format!("Created folder {}", result.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn apply_folder_rename_result(&mut self, result: FolderRenameResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.old_folder.clone()]);
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for folder rename", StatusTone::Error);
                    return;
                };
                for entry in result.entries {
                    let old_relative = result.old_folder.join(
                        entry.relative_path
                            .strip_prefix(&result.new_folder)
                            .unwrap_or(entry.relative_path.as_path()),
                    );
                    self.update_cached_entry(&source, &old_relative, entry.clone());
                }
                self.remap_folder_state(&result.old_folder, &result.new_folder);
                self.remap_manual_folders(&result.old_folder, &result.new_folder);
                self.refresh_folder_browser();
                self.focus_folder_by_path(&result.new_folder);
                self.set_status(
                    format!("Renamed folder to {}", result.new_folder.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn apply_folder_delete_result(&mut self, result: FolderDeleteResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                let source = SampleSource {
                    id: result.source_id.clone(),
                    root: result.source_root.clone(),
                };
                self.apply_deleted_folder_state(
                    &source,
                    &result.relative_path,
                    result.next_focus.as_deref(),
                    &result.entries,
                );
                if let Some(staged) = result.staged {
                    let before = self.capture_meaningful_ui_snapshot();
                    let after = self.capture_meaningful_ui_snapshot();
                    let entry = self.deleted_folder_undo_entry(
                        source,
                        result.staging_root,
                        staged,
                        result.entries,
                        result.next_focus,
                    );
                    self.push_undo_entry(AppController::attach_meaningful_ui_restore(
                        entry, before, after,
                    ));
                }
                self.set_status(
                    format!("Deleted folder {}", result.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn apply_selection_edit_commit_result(&mut self, result: SelectionEditCommitResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for edit", StatusTone::Error);
                    return;
                };
                if let Some(entry) = result.entry {
                    self.update_cached_entry(&source, &result.relative_path, entry);
                }
                self.clear_loaded_waveform_after_disk_edit();
                self.refresh_waveform_for_sample(&source, &result.relative_path);
                self.restore_selection_edit_visuals(result.preserve_selection, result.visual);
                self.queue_selection_edit_playback(
                    &crate::app::controller::library::selection_edits::SelectionTarget {
                        source: source.clone(),
                        relative_path: result.relative_path.clone(),
                        absolute_path: result.absolute_path.clone(),
                        selection: self
                            .ui
                            .waveform
                            .edit_selection
                            .or(self.ui.waveform.selection)
                            .unwrap_or_else(|| crate::selection::SelectionRange::new(0.0, 1.0)),
                    },
                    &result.playback,
                );
                self.maybe_trigger_pending_playback();
                if result.clear_edit_fades
                    && let Some(selection) = self.ui.waveform.edit_selection
                {
                    let cleared = selection.clear_fades().with_gain(1.0);
                    self.selection_state.edit_range.set_range(Some(cleared));
                    self.apply_edit_selection(Some(cleared));
                    self.record_edit_selection_apply_flash();
                }
                if result.clear_duplicate_cleanup {
                    self.clear_waveform_slices();
                    self.focus_waveform_context();
                }
                if let Some(backup) = result.backup {
                    self.push_undo_entry(self.selection_edit_undo_entry(
                        format!("{} {}", result.action_label, result.relative_path.display()),
                        result.source_id,
                        result.relative_path.clone(),
                        result.absolute_path,
                        backup,
                    ));
                }
                self.set_status(result.status_message, StatusTone::Info);
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    fn apply_waveform_slide_commit_result(&mut self, result: WaveformSlideCommitResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for waveform slide", StatusTone::Error);
                    return;
                };
                if let Some(entry) = result.entry {
                    self.update_cached_entry(&source, &result.relative_path, entry);
                }
                self.refresh_waveform_for_sample(&source, &result.relative_path);
                if let Some(backup) = result.backup {
                    self.push_undo_entry(self.selection_edit_undo_entry(
                        format!("Circular slide {}", result.relative_path.display()),
                        result.source_id,
                        result.relative_path.clone(),
                        result.absolute_path,
                        backup,
                    ));
                }
                self.set_status(
                    format!("Slid sample {}", result.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                if let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                {
                    self.refresh_waveform_for_sample(&source, &result.relative_path);
                }
                self.set_status(err, StatusTone::Error);
            }
        }
    }

    fn restore_deferred_entry(&mut self, pending: DeferredUndo<AppController>) {
        match pending.direction {
            UndoDirection::Undo => {
                self.history.undo_stack.restore_undo_entry(pending.entry);
            }
            UndoDirection::Redo => {
                self.history.undo_stack.restore_redo_entry(pending.entry);
            }
        }
    }
}
