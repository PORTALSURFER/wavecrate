use radiant::runtime::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{
    CACHE_FORMAT_VERSION, CACHE_FORMAT_VERSION_V2,
    identity::{
        CacheIdentity, cache_path_for_identity, playback_sidecar_path, playback_sidecar_valid,
    },
};
use crate::native_app::waveform::audio_file::{
    PersistedPlaybackCacheFile, WaveformFile, content_revision_for_audio_bytes,
};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CachedWaveformFileV2 {
    pub(super) version: u32,
    pub(super) path: PathBuf,
    pub(super) file_len: u64,
    pub(super) modified_ns: u128,
    pub(super) content_revision: u64,
    pub(super) sample_rate: u32,
    pub(super) channels: usize,
    pub(super) frames: usize,
    pub(super) summary: CachedGpuSignalSummary,
    pub(super) playback_samples: Option<Vec<f32>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) struct CachedPlaybackCacheFile {
    pub(super) sample_count: u64,
    pub(super) byte_len: u64,
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
}

impl CachedWaveformFileV2 {
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
            playback_samples: self.playback_samples.map(Arc::from),
            playback_cache_file: None,
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
        if !self.matches_identity(&path, &identity) || self.playback_samples.is_none() {
            return None;
        }
        Some(WaveformFile {
            path,
            audio_bytes: Arc::from([]),
            playback_samples: self.playback_samples.map(Arc::from),
            playback_cache_file: None,
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn matches_identity(&self, path: &Path, identity: &CacheIdentity) -> bool {
        self.version == CACHE_FORMAT_VERSION_V2
            && self.path == path
            && self.file_len == identity.file_len
            && self.modified_ns == identity.modified_ns
            && self.sample_rate != 0
            && self.channels != 0
            && self.frames != 0
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
