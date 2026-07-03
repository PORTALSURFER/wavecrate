use radiant::runtime::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::identity::{
    CacheIdentity, cache_path_for_identity, playback_sidecar_path, playback_sidecar_valid,
};
use crate::native_app::waveform::audio_file::{
    PersistedPlaybackCacheFile, PersistedPlaybackDescriptor, WaveformFile,
    content_revision_for_audio_bytes,
};

pub(super) const CACHE_FORMAT_VERSION: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CachedWaveformFile {
    version: u32,
    path: PathBuf,
    file_len: u64,
    modified_ns: u128,
    content_revision: u64,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    summary: CachedGpuSignalSummary,
    pub(super) playback_cache: Option<CachedPlaybackCacheFile>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct CachedPlaybackCacheFile {
    pub(super) sample_count: u64,
    pub(super) byte_len: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CachedPlaybackDescriptor {
    version: u32,
    path: PathBuf,
    file_len: u64,
    modified_ns: u128,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    playback_cache: CachedPlaybackCacheFile,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CachedGpuSignalSummary {
    frames: usize,
    band_count: usize,
    levels: Vec<CachedGpuSignalSummaryLevel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedGpuSignalSummaryLevel {
    bucket_frames: usize,
    buckets: Vec<CachedGpuSignalSummaryBucket>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct CachedGpuSignalSummaryBucket {
    min: f32,
    max: f32,
}

impl CachedWaveformFile {
    pub(super) fn from_waveform_file(
        file: &WaveformFile,
        identity: &CacheIdentity,
        playback_cache: Option<CachedPlaybackCacheFile>,
    ) -> Self {
        Self {
            version: CACHE_FORMAT_VERSION,
            path: file.path.clone(),
            file_len: identity.file_len,
            modified_ns: identity.modified_ns,
            content_revision: file.content_revision,
            sample_rate: file.sample_rate,
            channels: file.channels,
            frames: file.frames,
            summary: CachedGpuSignalSummary::from_summary(&file.gpu_signal_summary),
            playback_cache,
        }
    }

    pub(super) fn into_waveform_file(
        self,
        path: PathBuf,
        audio_bytes: Arc<[u8]>,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity)
            || self.content_revision != content_revision_for_audio_bytes(&audio_bytes)
        {
            return None;
        }
        Some(WaveformFile {
            path,
            audio_bytes,
            playback_samples: None,
            playback_cache_file: None,
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    pub(super) fn into_summary_waveform_file(
        self,
        path: PathBuf,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity) {
            return None;
        }
        let cache_path = cache_path_for_identity(&path, &identity).ok()?;
        Some(WaveformFile {
            path,
            audio_bytes: Arc::from([]),
            playback_samples: None,
            playback_cache_file: self
                .playback_cache
                .as_ref()
                .and_then(|_| self.playback_cache_file(&cache_path)),
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    pub(super) fn into_playback_ready_waveform_file(
        self,
        path: PathBuf,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity) || self.playback_cache.is_none() {
            return None;
        }
        let cache_path = cache_path_for_identity(&path, &identity).ok()?;
        let playback_cache_file = self.playback_cache_file(&cache_path)?;
        Some(WaveformFile {
            path,
            audio_bytes: Arc::from([]),
            playback_samples: None,
            playback_cache_file: Some(playback_cache_file),
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn matches_identity(&self, path: &Path, identity: &CacheIdentity) -> bool {
        self.version == CACHE_FORMAT_VERSION
            && self.path == path
            && self.file_len == identity.file_len
            && self.modified_ns == identity.modified_ns
            && self.sample_rate != 0
            && self.channels != 0
            && self.frames != 0
    }

    pub(super) fn playback_cache_file(
        &self,
        cache_path: &Path,
    ) -> Option<PersistedPlaybackCacheFile> {
        let playback_cache = self.playback_cache.as_ref()?;
        let sidecar_path = playback_sidecar_path(cache_path);
        if !playback_sidecar_valid(&sidecar_path, playback_cache.sample_count)
            || playback_cache.byte_len
                != playback_cache
                    .sample_count
                    .saturating_mul(std::mem::size_of::<f32>() as u64)
        {
            return None;
        }
        PersistedPlaybackCacheFile::new(sidecar_path, playback_cache.sample_count)
    }

    pub(super) fn into_moved_path(
        mut self,
        old_path: &Path,
        new_path: &Path,
        identity: &CacheIdentity,
    ) -> Option<Self> {
        if !self.matches_identity(old_path, identity) {
            return None;
        }
        self.path = new_path.to_path_buf();
        Some(self)
    }

    pub(super) fn clear_playback_cache(&mut self) {
        self.playback_cache = None;
    }
}

impl CachedPlaybackDescriptor {
    pub(super) fn from_cached_waveform_file(cached: &CachedWaveformFile) -> Option<Self> {
        Some(Self {
            version: cached.version,
            path: cached.path.clone(),
            file_len: cached.file_len,
            modified_ns: cached.modified_ns,
            sample_rate: cached.sample_rate,
            channels: cached.channels,
            frames: cached.frames,
            playback_cache: cached.playback_cache.clone()?,
        })
    }

    pub(super) fn into_playback_descriptor(
        self,
        path: PathBuf,
        identity: CacheIdentity,
        cache_path: &Path,
    ) -> Option<PersistedPlaybackDescriptor> {
        if self.version != CACHE_FORMAT_VERSION
            || self.path != path
            || self.file_len != identity.file_len
            || self.modified_ns != identity.modified_ns
            || self.sample_rate == 0
            || self.channels == 0
            || self.frames == 0
        {
            return None;
        }
        let sidecar_path = playback_sidecar_path(cache_path);
        if !playback_sidecar_valid(&sidecar_path, self.playback_cache.sample_count)
            || self.playback_cache.byte_len
                != self
                    .playback_cache
                    .sample_count
                    .saturating_mul(std::mem::size_of::<f32>() as u64)
        {
            return None;
        }
        PersistedPlaybackDescriptor::new(
            path,
            PersistedPlaybackCacheFile::new(sidecar_path, self.playback_cache.sample_count)?,
            self.sample_rate,
            self.channels,
            self.frames,
        )
    }
}

impl CachedGpuSignalSummary {
    pub(super) fn from_summary(summary: &GpuSignalSummary) -> Self {
        Self {
            frames: summary.frames,
            band_count: summary.band_count,
            levels: summary
                .levels
                .iter()
                .map(CachedGpuSignalSummaryLevel::from_level)
                .collect(),
        }
    }

    fn into_summary(self) -> Option<GpuSignalSummary> {
        if self.frames == 0 || self.band_count == 0 || self.levels.is_empty() {
            return None;
        }
        let mut levels = Vec::with_capacity(self.levels.len());
        for level in self.levels {
            levels.push(level.into_level(self.band_count)?);
        }
        Some(GpuSignalSummary {
            frames: self.frames,
            band_count: self.band_count,
            levels,
        })
    }
}

impl CachedGpuSignalSummaryLevel {
    fn from_level(level: &GpuSignalSummaryLevel) -> Self {
        Self {
            bucket_frames: level.bucket_frames,
            buckets: level
                .buckets
                .iter()
                .map(|bucket| CachedGpuSignalSummaryBucket {
                    min: bucket.min,
                    max: bucket.max,
                })
                .collect(),
        }
    }

    fn into_level(self, band_count: usize) -> Option<GpuSignalSummaryLevel> {
        if self.bucket_frames == 0
            || self.buckets.is_empty()
            || !self.buckets.len().is_multiple_of(band_count)
        {
            return None;
        }
        let buckets = self
            .buckets
            .into_iter()
            .map(|bucket| {
                (bucket.min.is_finite() && bucket.max.is_finite()).then_some(
                    GpuSignalSummaryBucket {
                        min: bucket.min,
                        max: bucket.max,
                    },
                )
            })
            .collect::<Option<Vec<_>>>()?;
        Some(GpuSignalSummaryLevel {
            bucket_frames: self.bucket_frames,
            buckets: buckets.into(),
        })
    }
}
