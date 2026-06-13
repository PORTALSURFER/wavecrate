//! Audio, playback, recording-waveform, and slice-export dispatch state.

use super::*;
use std::path::Path;

impl ControllerJobs {
    /// Return the in-flight audio load request, if any.
    pub(in super::super::super) fn pending_audio(&self) -> Option<PendingAudio> {
        self.pending_audio.clone()
    }

    /// Replace the active audio load request.
    pub(in super::super::super) fn set_pending_audio(&mut self, pending: Option<PendingAudio>) {
        self.pending_audio = pending;
    }

    /// Return the staged audio payload waiting for waveform visuals, if any.
    pub(in super::super::super) fn staged_audio_handoff(&self) -> Option<StagedAudioHandoff> {
        self.staged_audio_handoff.clone()
    }

    /// Replace the staged audio payload waiting for waveform visuals.
    pub(in super::super::super) fn set_staged_audio_handoff(
        &mut self,
        handoff: Option<StagedAudioHandoff>,
    ) {
        self.staged_audio_handoff = handoff;
    }

    /// Return the in-flight playback request, if any.
    pub(in super::super::super) fn pending_playback(&self) -> Option<PendingPlayback> {
        self.pending_playback.clone()
    }

    /// Replace the active playback request.
    pub(in super::super::super) fn set_pending_playback(
        &mut self,
        pending: Option<PendingPlayback>,
    ) {
        self.pending_playback = pending;
    }

    /// Return the in-flight recording waveform refresh request, if any.
    pub(in super::super::super) fn pending_recording_waveform(
        &self,
    ) -> Option<PendingRecordingWaveform> {
        self.pending_recording_waveform.clone()
    }

    /// Replace the active recording waveform refresh request.
    pub(in super::super::super) fn set_pending_recording_waveform(
        &mut self,
        pending: Option<PendingRecordingWaveform>,
    ) {
        self.pending_recording_waveform = pending;
    }

    /// Return the in-flight slice-batch export request, if any.
    pub(in super::super::super) fn pending_slice_batch_export(
        &self,
    ) -> Option<PendingSliceBatchExport> {
        self.pending_slice_batch_export.clone()
    }

    /// Replace the active slice-batch export request.
    pub(in super::super::super) fn set_pending_slice_batch_export(
        &mut self,
        pending: Option<PendingSliceBatchExport>,
    ) {
        self.pending_slice_batch_export = pending;
    }

    /// Return whether the active slice-batch export still matches the provided waveform.
    pub(in super::super::super) fn pending_slice_batch_export_matches(
        &self,
        request_id: u64,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> bool {
        self.pending_slice_batch_export
            .as_ref()
            .is_some_and(|pending| {
                pending.request_id == request_id
                    && &pending.source_id == source_id
                    && pending.relative_path == relative_path
            })
    }

    /// Clear the active slice-batch export request when the request id still matches.
    pub(in super::super::super) fn clear_pending_slice_batch_export(&mut self, request_id: u64) {
        if self
            .pending_slice_batch_export
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id)
        {
            self.pending_slice_batch_export = None;
        }
    }

    /// Queue one audio-load job after publishing the latest request id for staleness checks.
    pub(in super::super::super) fn send_audio_job(&self, job: AudioLoadJob) -> Result<(), ()> {
        self.audio_loader.publish_latest_request_id(job.request_id);
        self.audio_job_tx.send(job).map_err(|_| ())
    }

    /// Send a background recording waveform refresh job.
    pub(in super::super::super) fn send_recording_waveform_job(&self, job: RecordingWaveformJob) {
        self.recording_waveform_job_tx.send(job);
    }
}
