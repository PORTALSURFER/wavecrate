//! Selection edit and waveform slide result application helpers.

use super::*;

impl AppController {
    pub(super) fn apply_selection_edit_commit_result(&mut self, result: SelectionEditCommitResult) {
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
                    self.trigger_analysis_for_changed_entry(&source, &entry, false);
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

    pub(super) fn apply_waveform_slide_commit_result(&mut self, result: WaveformSlideCommitResult) {
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
                    self.trigger_analysis_for_changed_entry(&source, &entry, false);
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
}
