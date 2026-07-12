use super::*;

impl AppController {
    /// Crop the loaded sample to the active selection range and refresh caches/exports.
    pub(crate) fn crop_waveform_selection(&mut self) -> Result<(), String> {
        if !cfg!(test) {
            return self.queue_selection_edit_commit(
                "Cropped selection",
                format!(
                    "Cropped selection {}",
                    self.selection_target()?.relative_path.display()
                ),
                false,
                false,
                false,
                SelectionEditWorkerOp::Crop,
            );
        }
        let result = self.apply_selection_edit("Cropped selection", false, crop_buffer);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    /// Write the cropped selection to a new sample file alongside the original.
    pub(crate) fn crop_waveform_selection_to_new_sample(&mut self) -> Result<(), String> {
        let session = self.begin_crop_new_sample_session()?;
        let request_id = self.runtime.jobs.next_selection_export_request_id();
        self.begin_pending_sample_creation_transaction(
            crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                request_id,
            },
            "Cropped to new sample",
        );
        self.queue_selection_export_job(
            crate::app::controller::jobs::SelectionExportJob::CropNewSample {
                request_id,
                snapshot: self.capture_selection_export_snapshot(
                    session.target.selection,
                    Some(session.tag),
                )?,
                playback: crate::app::controller::jobs::SelectionExportPlaybackState {
                    was_playing: session.playback.was_playing,
                    was_looping: session.playback.was_looping,
                    start_override: session.playback.start_override,
                },
            },
        );
        self.set_status("Cropping selection to new sample...", StatusTone::Busy);
        Ok(())
    }
}
