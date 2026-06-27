use super::super::super::DragDropController;
use super::worker::run_folder_move_task;
use crate::app::controller::AppController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{FolderMoveRequest, FolderMoveResult, FolderSampleMoveResult};
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use crate::sample_sources::{SampleSource, WavEntry};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};
use tracing::{info, warn};

impl DragDropController<'_> {
    /// Apply a completed background in-source sample move job.
    pub(crate) fn apply_folder_sample_move_result(&mut self, result: FolderSampleMoveResult) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.source_id)
            .cloned()
        else {
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        let mut updates = Vec::new();
        for entry in &result.moved {
            let old_entry = WavEntry {
                relative_path: entry.old_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                sound_type: entry.sound_type,
                locked: entry.locked,
                missing: false,
                last_played_at: entry.last_played_at,
                last_curated_at: entry.last_curated_at,
                user_tag: entry.user_tag.clone(),
                tag_named: false,
                normal_tags: entry.normal_tags.clone(),
            };
            let new_entry = WavEntry {
                relative_path: entry.new_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                sound_type: entry.sound_type,
                locked: entry.locked,
                missing: false,
                last_played_at: entry.last_played_at,
                last_curated_at: entry.last_curated_at,
                user_tag: entry.user_tag.clone(),
                tag_named: false,
                normal_tags: entry.normal_tags.clone(),
            };
            updates.push((old_entry, new_entry));
        }
        if !updates.is_empty() {
            self.apply_folder_entry_updates(&source, &updates);
        }
        let moved = result.moved.len();
        if moved == 0 {
            if let Some(err) = result.errors.first() {
                let message = if result.cancelled {
                    format!("{err} (cancelled)")
                } else {
                    err.clone()
                };
                self.set_status(message, StatusTone::Warning);
            } else if result.cancelled {
                self.set_status("Move cancelled", StatusTone::Warning);
            } else {
                self.set_status("No samples moved", StatusTone::Warning);
            }
            return;
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = format!("Moved {moved} sample(s)");
        if !result.errors.is_empty() {
            message.push_str(&format!(" with {} error(s)", result.errors.len()));
        }
        if result.cancelled {
            message.push_str(" (cancelled)");
        }
        self.set_status(message, tone);
        for err in &result.errors {
            warn!(error = %err, moved, "Folder move error");
        }
        info!(
            "Folder move completed: {} moved, {} errors",
            moved,
            result.errors.len()
        );
    }

    /// Apply a completed background folder move job.
    pub(crate) fn apply_folder_move_result(&mut self, result: FolderMoveResult) {
        self.apply_folder_move_result_inner(result, true);
    }

    fn apply_folder_move_result_without_undo(&mut self, result: FolderMoveResult) {
        self.apply_folder_move_result_inner(result, false);
    }

    fn apply_folder_move_result_inner(&mut self, result: FolderMoveResult, register_undo: bool) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.source_id)
            .cloned()
        else {
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        if !result.folder_moved {
            if result.errors.is_empty() {
                if result.cancelled {
                    self.set_status("Move cancelled", StatusTone::Warning);
                } else {
                    self.set_status("No folders moved", StatusTone::Warning);
                }
            } else if result.cancelled {
                self.set_status("Move cancelled", StatusTone::Warning);
            } else {
                self.set_status(result.errors[0].clone(), StatusTone::Error);
            }
            for err in &result.errors {
                warn!(
                    error = %err,
                    folder_moved = result.folder_moved,
                    cancelled = result.cancelled,
                    "Folder move error"
                );
            }
            return;
        }
        let mut updates = Vec::new();
        for entry in &result.moved {
            let old_entry = WavEntry {
                relative_path: entry.old_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                sound_type: entry.sound_type,
                locked: entry.locked,
                missing: false,
                last_played_at: entry.last_played_at,
                last_curated_at: entry.last_curated_at,
                user_tag: entry.user_tag.clone(),
                tag_named: false,
                normal_tags: entry.normal_tags.clone(),
            };
            let new_entry = WavEntry {
                relative_path: entry.new_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                sound_type: entry.sound_type,
                locked: entry.locked,
                missing: false,
                last_played_at: entry.last_played_at,
                last_curated_at: entry.last_curated_at,
                user_tag: entry.user_tag.clone(),
                tag_named: false,
                normal_tags: entry.normal_tags.clone(),
            };
            updates.push((old_entry, new_entry));
        }
        if !updates.is_empty() {
            self.apply_folder_entry_updates(&source, &updates);
        }
        self.remap_folder_state(&result.old_folder, &result.new_folder);
        self.remap_manual_folders(&result.old_folder, &result.new_folder);
        self.focus_drop_target_folder(&result.new_folder);
        if register_undo {
            self.push_undo_entry(folder_move_undo_entry(
                source,
                result.old_folder.clone(),
                result.new_folder.clone(),
            ));
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = format!("Moved folder to {}", result.new_folder.display());
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
                new_folder = %result.new_folder.display(),
                cancelled = result.cancelled,
                "Folder move error"
            );
        }
    }
}

fn folder_move_undo_entry(
    source: SampleSource,
    old_folder: PathBuf,
    new_folder: PathBuf,
) -> UndoEntry<AppController> {
    let label = format!("Move folder {}", old_folder.display());
    let undo_source = source.clone();
    let undo_old_folder = old_folder.clone();
    let undo_new_folder = new_folder.clone();
    let redo_source = source;
    UndoEntry::new(
        label,
        move |controller| {
            let target_parent = undo_old_folder.parent().unwrap_or_else(|| Path::new(""));
            apply_folder_move_for_undo(controller, &undo_source, &undo_new_folder, target_parent)
        },
        move |controller| {
            let target_parent = new_folder.parent().unwrap_or_else(|| Path::new(""));
            apply_folder_move_for_undo(controller, &redo_source, &old_folder, target_parent)
        },
    )
}

fn apply_folder_move_for_undo(
    controller: &mut AppController,
    source: &SampleSource,
    folder: &Path,
    target_folder: &Path,
) -> Result<UndoExecution, String> {
    let request = FolderMoveRequest {
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        folder: folder.to_path_buf(),
        target_folder: target_folder.to_path_buf(),
    };
    let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);
    if !result.folder_moved {
        return Err(result
            .errors
            .first()
            .cloned()
            .unwrap_or_else(|| String::from("Folder move did not complete")));
    }
    let mut drag_drop = DragDropController::new(controller);
    drag_drop.apply_folder_move_result_without_undo(result);
    Ok(UndoExecution::Applied)
}
