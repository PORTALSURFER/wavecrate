//! Clipboard paste reporting and deferred undo application helpers.

use super::*;

impl AppController {
    pub(super) fn apply_clipboard_paste_result(&mut self, result: ClipboardPasteResult) {
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
                        self.trigger_analysis_for_added_sample(
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

    pub(super) fn apply_undo_file_result(&mut self, result: UndoFileOpResult) {
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
                normal_tags,
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
                    sound_type: None,
                    locked: false,
                    missing: false,
                    last_played_at: *last_played_at,
                    user_tag: None,
                    normal_tags: normal_tags.clone(),
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
                normal_tags,
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
                        sound_type: None,
                        locked: false,
                        missing: false,
                        last_played_at: *last_played_at,
                        user_tag: None,
                        normal_tags: normal_tags.clone(),
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
