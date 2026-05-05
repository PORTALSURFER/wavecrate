//! Shared routing helpers for background-job polling.

use super::*;
use crate::app::controller::state::audio::{PendingAudio, PendingRecordingWaveform};
use crate::sample_sources::SourceId;
use std::path::Path;

/// Maps progress-overlay tasks to the worker cancellation path they control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CancelRequestAction {
    TrashMove,
    Scan,
    Analysis,
    FileOps,
    None,
}

/// Resolve the cancellation action for the active progress task, if any.
pub(super) fn cancel_request_action(task: Option<ProgressTaskKind>) -> CancelRequestAction {
    match task {
        Some(ProgressTaskKind::TrashMove) => CancelRequestAction::TrashMove,
        Some(ProgressTaskKind::Scan) => CancelRequestAction::Scan,
        Some(ProgressTaskKind::Analysis) => CancelRequestAction::Analysis,
        Some(ProgressTaskKind::FileOps) => CancelRequestAction::FileOps,
        _ => CancelRequestAction::None,
    }
}

/// Return whether an audio-load completion still matches the current pending request.
pub(super) fn pending_audio_matches(
    pending: &PendingAudio,
    request_id: u64,
    source_id: &SourceId,
    relative_path: &Path,
) -> bool {
    request_id == pending.request_id
        && source_id == &pending.source_id
        && relative_path == pending.relative_path
}

/// Return whether a recording-waveform completion still matches the current pending request.
pub(super) fn pending_recording_waveform_matches(
    pending: &PendingRecordingWaveform,
    request_id: u64,
    source_id: &SourceId,
    relative_path: &Path,
) -> bool {
    request_id == pending.request_id
        && source_id == &pending.source_id
        && relative_path == pending.relative_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_request_action_maps_supported_progress_tasks() {
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::TrashMove)),
            CancelRequestAction::TrashMove
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::Scan)),
            CancelRequestAction::Scan
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::Analysis)),
            CancelRequestAction::Analysis
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::FileOps)),
            CancelRequestAction::FileOps
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::WavLoad)),
            CancelRequestAction::None
        );
        assert_eq!(cancel_request_action(None), CancelRequestAction::None);
    }

    #[test]
    fn pending_audio_matches_requires_request_source_and_path_match() {
        let source = SourceId::from_string("source-a");
        let pending = PendingAudio {
            request_id: 42,
            source_id: source.clone(),
            root: std::path::PathBuf::from("/tmp/source"),
            relative_path: std::path::PathBuf::from("kick.wav"),
            intent: crate::app::controller::state::audio::AudioLoadIntent::Selection,
        };

        assert!(pending_audio_matches(
            &pending,
            42,
            &source,
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            41,
            &source,
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            42,
            &SourceId::from_string("source-b"),
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            42,
            &source,
            Path::new("snare.wav")
        ));
    }

    #[test]
    fn pending_recording_waveform_matches_requires_request_source_and_path_match() {
        let source = SourceId::from_string("source-a");
        let pending = PendingRecordingWaveform {
            request_id: 77,
            source_id: source.clone(),
            relative_path: std::path::PathBuf::from("recording.wav"),
            absolute_path: std::path::PathBuf::from("/tmp/source/recording.wav"),
        };

        assert!(pending_recording_waveform_matches(
            &pending,
            77,
            &source,
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            76,
            &source,
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            77,
            &SourceId::from_string("source-b"),
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            77,
            &source,
            Path::new("other.wav")
        ));
    }
}
