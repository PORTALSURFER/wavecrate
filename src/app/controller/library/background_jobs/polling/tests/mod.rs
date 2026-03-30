use super::*;
use crate::app::controller::AppController;
use crate::app::controller::jobs::{
    JobMessage, SelectionExportMessage,
};
use crate::app::controller::library::analysis_jobs::AnalysisJobMessage;
use crate::app::controller::state::audio::PendingAudio;
use crate::app::controller::AudioLoadIntent;
use crate::app::controller::playback::audio_loader::{AudioLoadOutcome, AudioLoadResult};
use crate::sample_sources::SampleSource;
use std::path::Path;
use std::sync::Arc;

mod audio;
mod dispatch;
mod file_ops;
mod poll;
mod similarity;

fn decode_audio_outcome(
    controller: &AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> AudioLoadOutcome {
    let metadata = controller
        .current_file_metadata(source, relative_path)
        .expect("metadata");
    let bytes: Arc<[u8]> = controller
        .read_waveform_bytes(source, relative_path)
        .expect("waveform bytes")
        .into();
    let decoded = Arc::new(
        controller
            .sample_view
            .renderer
            .decode_from_bytes(bytes.as_ref())
            .expect("decoded waveform"),
    );
    AudioLoadOutcome {
        decoded,
        bytes,
        metadata,
        transients: None,
        stretched: false,
    }
}
