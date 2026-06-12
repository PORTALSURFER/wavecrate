use crate::app::controller::library::wavs::WaveformRenderMeta;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::sample_sources::SourceId;
use crate::waveform::{DecodedWaveform, WaveformImage};
use radiant::gui::types::ImageRgba;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug)]
pub(crate) struct AudioLoadOutcome {
    pub decoded: Arc<DecodedWaveform>,
    pub bytes: Arc<[u8]>,
    pub audio_path: Option<PathBuf>,
    pub metadata: FileMetadata,
    pub transients: Option<Arc<[f32]>>,
    pub stretched: bool,
}

#[derive(Debug)]
/// Deferred initial waveform visual payload produced off the controller thread.
pub(crate) struct AudioVisualResult {
    pub request_id: u64,
    pub source_id: SourceId,
    pub relative_path: PathBuf,
    pub metadata: FileMetadata,
    pub cache_token: u64,
    pub transients: Arc<[f32]>,
    pub image: Option<WaveformImage>,
    pub projected_image: Option<Arc<ImageRgba>>,
    pub render_meta: Option<WaveformRenderMeta>,
    pub stretched: bool,
}

#[derive(Debug)]
pub(crate) enum AudioLoadError {
    Missing(String),
    Failed(String),
}

#[derive(Debug)]
/// Deferred transient-marker payload for an already-delivered audio load.
pub(crate) struct AudioTransientResult {
    pub request_id: u64,
    pub source_id: SourceId,
    pub relative_path: PathBuf,
    pub metadata: FileMetadata,
    pub cache_token: u64,
    pub transients: Arc<[f32]>,
    pub stretched: bool,
}

#[derive(Debug)]
/// Audio loader worker message stream: primary load completion plus deferred transients.
pub(crate) enum AudioLoadResult {
    Primary {
        request_id: u64,
        source_id: SourceId,
        relative_path: PathBuf,
        result: Result<AudioLoadOutcome, AudioLoadError>,
    },
    Transients(AudioTransientResult),
    Visual(AudioVisualResult),
}
