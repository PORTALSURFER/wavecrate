use super::super::super::DragDropController;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{FolderMoveResult, FolderSampleMoveResult};
use crate::sample_sources::WavEntry;
use tracing::info;

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
                missing: false,
                last_played_at: entry.last_played_at,
            };
            let new_entry = WavEntry {
                relative_path: entry.new_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                missing: false,
                last_played_at: entry.last_played_at,
            };
            updates.push((old_entry, new_entry));
        }
        if !updates.is_empty() {
            self.apply_folder_entry_updates(&source, &updates);
        }
        let moved = result.moved.len();
        if moved == 0 && result.errors.is_empty() {
            if result.cancelled {
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
            eprintln!("Folder move error: {err}");
        }
        info!(
            "Folder move completed: {} moved, {} errors",
            moved,
            result.errors.len()
        );
    }

    /// Apply a completed background folder move job.
    pub(crate) fn apply_folder_move_result(&mut self, result: FolderMoveResult) {
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
                eprintln!("Folder move error: {err}");
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
                missing: false,
                last_played_at: entry.last_played_at,
            };
            let new_entry = WavEntry {
                relative_path: entry.new_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                missing: false,
                last_played_at: entry.last_played_at,
            };
            updates.push((old_entry, new_entry));
        }
        if !updates.is_empty() {
            self.apply_folder_entry_updates(&source, &updates);
        }
        self.remap_folder_state(&result.old_folder, &result.new_folder);
        self.remap_manual_folders(&result.old_folder, &result.new_folder);
        self.focus_drop_target_folder(&result.new_folder);
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
            eprintln!("Folder move error: {err}");
        }
    }
}
