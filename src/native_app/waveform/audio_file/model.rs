use radiant::runtime::GpuSignalSummary;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use super::visual_bands::VisualBandNormalization;

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
    pub(in crate::native_app::waveform) visual_band_normalization: VisualBandNormalization,
    pub(in crate::native_app::waveform) gpu_signal_summary: Arc<GpuSignalSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PersistedPlaybackCacheFile {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) sample_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PersistedPlaybackDescriptor {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) cache_file: PersistedPlaybackCacheFile,
    pub(in crate::native_app) sample_rate: u32,
    pub(in crate::native_app) channels: usize,
    pub(in crate::native_app) frames: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformPlaybackReady {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) audio_bytes: Arc<[u8]>,
    pub(in crate::native_app) playback_samples: Arc<[f32]>,
    pub(in crate::native_app) sample_rate: u32,
    pub(in crate::native_app) channels: usize,
    pub(in crate::native_app) frames: usize,
    pub(in crate::native_app) source_len: u64,
    pub(in crate::native_app) source_modified: Option<SystemTime>,
}

impl PersistedPlaybackCacheFile {
    pub(in crate::native_app) fn new(path: PathBuf, sample_count: u64) -> Option<Self> {
        (sample_count > 0).then_some(Self { path, sample_count })
    }
}

impl PersistedPlaybackDescriptor {
    pub(in crate::native_app) fn new(
        path: PathBuf,
        cache_file: PersistedPlaybackCacheFile,
        sample_rate: u32,
        channels: usize,
        frames: usize,
    ) -> Option<Self> {
        (sample_rate != 0 && channels != 0 && frames != 0).then_some(Self {
            path,
            cache_file,
            sample_rate,
            channels,
            frames,
        })
    }

    pub(in crate::native_app) fn duration_seconds(&self) -> f32 {
        self.frames as f32 / self.sample_rate as f32
    }
}

impl WaveformFile {
    pub(in crate::native_app) fn clone_remapped_after_path_move(
        &self,
        old_path: &Path,
        new_path: &Path,
    ) -> Option<Self> {
        let mapped_path = remapped_waveform_path(&self.path, old_path, new_path)?;
        if mapped_path == self.path {
            return None;
        }
        let mut file = self.clone();
        file.apply_remapped_path(mapped_path);
        file.playback_cache_file = None;
        Some(file)
    }

    pub(in crate::native_app::waveform) fn rewrite_path_prefix(
        &mut self,
        old_path: &Path,
        new_path: &Path,
    ) -> bool {
        let Some(mapped_path) = remapped_waveform_path(&self.path, old_path, new_path) else {
            return false;
        };
        if mapped_path == self.path {
            return false;
        }
        self.apply_remapped_path(mapped_path);
        true
    }

    fn apply_remapped_path(&mut self, mapped_path: PathBuf) {
        self.path = mapped_path;
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            self.content_revision = content_revision_for_path_metadata(
                &self.path,
                &metadata,
                self.sample_rate,
                self.channels,
                self.frames,
            );
        }
    }

    pub(in crate::native_app::waveform) fn has_loaded_sample_metadata(&self) -> bool {
        !self.path.as_os_str().is_empty()
            && (!self.audio_bytes.is_empty()
                || self.playback_samples.is_some()
                || self.playback_cache_file.is_some()
                || self.file_backed_playback_metadata_available())
    }

    pub(in crate::native_app::waveform) fn file_backed_playback_metadata_available(&self) -> bool {
        self.audio_bytes.is_empty()
            && self.playback_samples.is_none()
            && self.playback_cache_file.is_none()
            && self.sample_rate != 0
            && self.channels != 0
            && self.frames != 0
    }

    pub(in crate::native_app) fn instant_audition_payload_available(&self) -> bool {
        self.playback_samples.is_some() || self.playback_cache_file.is_some()
    }

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

pub(super) fn content_revision_for_path_metadata(
    path: &Path,
    metadata: &std::fs::Metadata,
    sample_rate: u32,
    channels: usize,
    frames: usize,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified_ns(metadata).hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    channels.hash(&mut hasher);
    frames.hash(&mut hasher);
    hasher.finish().max(1)
}

fn modified_ns(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn remapped_waveform_path(path: &Path, old_path: &Path, new_path: &Path) -> Option<PathBuf> {
    if path == old_path {
        return Some(new_path.to_path_buf());
    }
    path.strip_prefix(old_path)
        .ok()
        .map(|relative| new_path.join(relative))
}
