use radiant::runtime::GpuSignalSummary;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
    time::SystemTime,
};

use super::{
    PreviewAuditionClip, WaveformFile,
    construction::waveform_file_from_mono_samples_with_progress_and_cancel,
    downmix::downmix_to_mono_with_progress_and_cancel,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::native_app) enum InstantWaveformPreviewTier {
    Head,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct InstantWaveformPreview {
    pub(in crate::native_app) file: Arc<WaveformFile>,
    pub(in crate::native_app) tier: InstantWaveformPreviewTier,
    pub(in crate::native_app) source_len: u64,
    pub(in crate::native_app) source_modified: Option<SystemTime>,
}

impl InstantWaveformPreview {
    pub(in crate::native_app) fn path(&self) -> &Path {
        &self.file.path
    }

    pub(in crate::native_app) fn byte_len(&self) -> usize {
        signal_summary_byte_len(self.file.gpu_signal_summary.as_ref())
    }

    #[cfg(test)]
    pub(in crate::native_app) fn matches_file(&self, path: &Path) -> bool {
        source_identity(path)
            .is_some_and(|identity| identity == (self.source_len, self.source_modified))
    }

    pub(in crate::native_app) fn matches_source_identity(
        &self,
        source_len: u64,
        source_modified: Option<SystemTime>,
    ) -> bool {
        self.source_len == source_len && self.source_modified == source_modified
    }
}

impl PartialEq for InstantWaveformPreview {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path()
            && self.tier == other.tier
            && self.source_len == other.source_len
            && self.source_modified == other.source_modified
    }
}

pub(in crate::native_app) fn instant_waveform_head_preview_from_clip(
    clip: PreviewAuditionClip,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<InstantWaveformPreview, String> {
    let mono = downmix_to_mono_with_progress_and_cancel(
        &clip.samples,
        clip.channels,
        clip.frames,
        0.0,
        0.18,
        progress,
        cancelled,
    )?;
    let mut file = waveform_file_from_mono_samples_with_progress_and_cancel(
        clip.path.clone(),
        Arc::from([]),
        clip.sample_rate,
        clip.channels,
        mono,
        progress,
        cancelled,
    )?;
    file.content_revision = instant_preview_content_revision(
        &clip.path,
        clip.source_len,
        clip.source_modified,
        clip.sample_rate,
        clip.channels,
        file.frames,
        InstantWaveformPreviewTier::Head,
    );
    Ok(InstantWaveformPreview {
        file: Arc::new(file),
        tier: InstantWaveformPreviewTier::Head,
        source_len: clip.source_len,
        source_modified: clip.source_modified,
    })
}

#[cfg(test)]
fn source_identity(path: &Path) -> Option<(u64, Option<SystemTime>)> {
    let metadata = path.metadata().ok()?;
    Some((metadata.len(), metadata.modified().ok()))
}

fn instant_preview_content_revision(
    path: &Path,
    source_len: u64,
    source_modified: Option<SystemTime>,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    tier: InstantWaveformPreviewTier,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    source_len.hash(&mut hasher);
    source_modified.and_then(system_time_ns).hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    channels.hash(&mut hasher);
    frames.hash(&mut hasher);
    tier.hash(&mut hasher);
    hasher.finish().max(1)
}

fn system_time_ns(time: SystemTime) -> Option<u128> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

fn signal_summary_byte_len(summary: &GpuSignalSummary) -> usize {
    summary
        .levels
        .iter()
        .map(|level| {
            level
                .buckets
                .len()
                .saturating_mul(std::mem::size_of::<radiant::runtime::GpuSignalSummaryBucket>())
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_waveform_preview_identity_rejects_changed_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("preview.wav");
        std::fs::write(&path, b"one").expect("write");
        let (source_len, source_modified) = source_identity(&path).expect("identity");
        let file = WaveformFile {
            path: path.clone(),
            audio_bytes: Arc::from([]),
            playback_samples: None,
            playback_cache_file: None,
            content_revision: 1,
            sample_rate: 48_000,
            channels: 1,
            frames: 1,
            visual_band_normalization:
                crate::native_app::waveform::audio_file::VisualBandNormalization::IDENTITY,
            gpu_signal_summary: Arc::new(GpuSignalSummary {
                frames: 1,
                band_count: 1,
                levels: Vec::new(),
            }),
        };
        let preview = InstantWaveformPreview {
            file: Arc::new(file),
            tier: InstantWaveformPreviewTier::Head,
            source_len,
            source_modified,
        };

        std::fs::write(&path, b"changed").expect("rewrite");

        assert!(!preview.matches_file(&path));
    }
}
