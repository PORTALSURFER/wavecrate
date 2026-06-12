use crate::app::controller::library::wavs::waveform_rendering::InitialWaveformRenderSpec;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::sample_sources::SourceId;
use crate::waveform::DecodedWaveform;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
/// Inputs required to compute deferred waveform visuals after primary delivery.
pub(crate) struct PendingVisualCompute {
    pub(super) request_id: u64,
    pub(super) source_id: SourceId,
    pub(super) relative_path: PathBuf,
    pub(super) metadata: FileMetadata,
    pub(super) cache_token: u64,
    pub(super) decoded: Arc<DecodedWaveform>,
    pub(super) render_spec: InitialWaveformRenderSpec,
    pub(super) known_transients: Option<Arc<[f32]>>,
    pub(super) stretched: bool,
}

#[derive(Clone)]
/// Inputs required to compute transient markers before visual preparation.
pub(crate) struct PendingTransientCompute {
    pub(super) request_id: u64,
    pub(super) source_id: SourceId,
    pub(super) relative_path: PathBuf,
    pub(super) metadata: FileMetadata,
    pub(super) cache_token: u64,
    pub(super) decoded: Arc<DecodedWaveform>,
    pub(super) stretched: bool,
}
