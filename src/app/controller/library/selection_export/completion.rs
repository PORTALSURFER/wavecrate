//! UI-thread completion handlers for finished selection exports.

use super::*;

impl AppController {
    /// Apply one completed selection clip export on the UI thread.
    pub(crate) fn apply_selection_clip_export_success(
        &mut self,
        success: SelectionClipExportSuccess,
    ) {
        let history_key =
            crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                request_id: success.request_id,
            };
        let history_source_id = success.source_id.clone();
        let history_relative_path = success.entry.relative_path.clone();
        let history_absolute_path = success.absolute_path.clone();
        let history_tag = success.entry.tag;
        self.record_selection_export_timings("clip", &success.entry.relative_path, success.timings);
        let source = SampleSource {
            id: success.source_id.clone(),
            root: success.source_root.clone(),
        };
        self.insert_cached_entry(&source, success.entry.clone());
        self.enqueue_similarity_for_new_sample(
            &source,
            &success.entry.relative_path,
            success.entry.file_size,
            success.entry.modified_ns,
        );
        match success.destination {
            SelectionClipDestination::Browser {
                keep_source_focused,
                ..
            }
            | SelectionClipDestination::Folder {
                keep_source_focused,
                ..
            } => {
                if !keep_source_focused {
                    self.ui.browser.selection.autoscroll = true;
                    self.selection_state.suppress_autoplay_once = true;
                    self.select_from_browser(&success.entry.relative_path);
                }
                self.set_status(
                    format!("Saved clip {}", success.entry.relative_path.display()),
                    StatusTone::Info,
                );
            }
            SelectionClipDestination::ExternalDrag => {
                self.finish_external_selection_drag_export(success);
            }
        }
        if let Err(err) = self.finish_pending_sample_creation_transaction(
            &history_key,
            history_source_id,
            history_relative_path,
            history_absolute_path,
            history_tag,
        ) {
            self.set_status(
                format!("Selection export undo failed: {err}"),
                StatusTone::Error,
            );
        }
    }

    /// Apply one completed crop-to-new-sample export on the UI thread.
    pub(crate) fn apply_selection_crop_export_success(
        &mut self,
        success: SelectionCropExportSuccess,
    ) {
        self.record_selection_export_timings(
            "crop_new_sample",
            &success.entry.relative_path,
            success.timings,
        );
        let source = SampleSource {
            id: success.source_id.clone(),
            root: success.source_root.clone(),
        };
        self.insert_cached_entry(&source, success.entry.clone());
        self.enqueue_similarity_for_new_sample(
            &source,
            &success.entry.relative_path,
            success.entry.file_size,
            success.entry.modified_ns,
        );
        if let Ok(backup) = undo::OverwriteBackup::capture_before(&success.absolute_path)
            && backup.capture_after(&success.absolute_path).is_ok()
        {
            self.push_undo_entry(self.crop_new_sample_undo_entry(
                format!(
                    "Cropped to new sample {}",
                    success.entry.relative_path.display()
                ),
                source.id.clone(),
                success.entry.relative_path.clone(),
                success.absolute_path.clone(),
                success.tag,
                backup,
            ));
        }
        self.ui.browser.selection.autoscroll = true;
        self.selection_state.suppress_autoplay_once = true;
        self.select_wav_by_path(&success.entry.relative_path);
        if success.playback.was_playing {
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: source.id.clone(),
                    relative_path: success.entry.relative_path.clone(),
                    looped: success.playback.was_looping,
                    start_override: success.playback.start_override,
                }));
        }
        self.focus_waveform();
        self.set_status(
            format!(
                "Cropped to new sample {}",
                success.entry.relative_path.display()
            ),
            StatusTone::Info,
        );
    }

    #[cfg(target_os = "windows")]
    fn finish_external_selection_drag_export(&mut self, success: SelectionClipExportSuccess) {
        let Some(request_id) = self.ui.drag.pending_external_selection_request_id else {
            return;
        };
        if request_id != success.request_id {
            return;
        }
        self.ui.drag.pending_external_selection_request_id = None;
        match self
            .drag_drop()
            .start_external_drag(&[success.absolute_path.clone()])
        {
            Ok(()) => {
                let label = format!(
                    "Drag {} to an external target",
                    success.entry.relative_path.display()
                );
                self.drag_drop().reset_drag();
                self.set_status(label, StatusTone::Info);
            }
            Err(err) => {
                self.drag_drop().reset_drag();
                self.set_status(err, StatusTone::Error);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn finish_external_selection_drag_export(&mut self, _success: SelectionClipExportSuccess) {}
}
