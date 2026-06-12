use radiant::runtime::GpuSignalSummary;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformFile {
    pub(in crate::native_app::waveform) path: PathBuf,
    pub(in crate::native_app::waveform) audio_bytes: Arc<[u8]>,
    pub(in crate::native_app::waveform) playback_samples: Option<Arc<[f32]>>,
    pub(in crate::native_app::waveform) playback_cache_file: Option<PersistedPlaybackCacheFile>,
    pub(in crate::native_app::waveform) content_revision: u64,
    pub(in crate::native_app::waveform) sample_rate: u32,
    pub(in crate::native_app::waveform) channels: usize,
    pub(in crate::native_app::waveform) frames: usize,
    pub(in crate::native_app::waveform) gpu_signal_summary: Arc<GpuSignalSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PersistedPlaybackCacheFile {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) sample_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformPlaybackReady {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) audio_bytes: Arc<[u8]>,
    pub(in crate::native_app) playback_samples: Arc<[f32]>,
    pub(in crate::native_app) sample_rate: u32,
    pub(in crate::native_app) channels: usize,
    pub(in crate::native_app) frames: usize,
}

impl PersistedPlaybackCacheFile {
    pub(in crate::native_app) fn new(path: PathBuf, sample_count: u64) -> Option<Self> {
        (sample_count > 0).then_some(Self { path, sample_count })
    }
}

impl WaveformFile {
    pub(in crate::native_app::waveform) fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        self.channels.hash(&mut hasher);
        hasher.finish()
    }

    pub(in crate::native_app::waveform) fn content_revision(&self) -> u64 {
        self.content_revision
    }
}
