use crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::sample_sources::SourceId;
use crate::waveform::DecodedWaveform;
use std::{path::PathBuf, sync::Arc};

pub(crate) struct AudioLoadJob {
    pub request_id: u64,
    pub source_id: SourceId,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub stretch_ratio: Option<f64>,
    pub render_spec: InitialWaveformRenderSpec,
    pub prepared: Option<PreparedAudioLoad>,
}

#[derive(Clone, Debug)]
/// Fully prepared in-memory audio payload queued back through the worker path.
pub(crate) struct PreparedAudioLoad {
    pub metadata: FileMetadata,
    pub decoded: Arc<DecodedWaveform>,
    pub bytes: Arc<[u8]>,
    pub transients: Arc<[f32]>,
    pub stretched: bool,
}
