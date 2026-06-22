use std::{fs, path::Path, sync::Arc};

use super::{PersistedPlaybackCacheFile, SimilarSectionsSource};
use crate::native_app::waveform::audio_file::read_wav_playback_samples;

impl SimilarSectionsSource {
    pub(super) fn load_samples(&self, path: &Path) -> Result<Arc<[f32]>, String> {
        match self {
            Self::InterleavedF32Samples(samples) => Ok(Arc::clone(samples)),
            Self::InterleavedF32File(cache_file) => read_interleaved_f32_file(cache_file),
            Self::WavBytes(bytes) => read_wav_playback_samples(bytes).map(Arc::from),
            Self::WavFile => read_wav_file(path),
        }
    }
}

fn read_wav_file(path: &Path) -> Result<Arc<[f32]>, String> {
    let bytes: Arc<[u8]> = fs::read(path)
        .map(Arc::from)
        .map_err(|err| format!("failed to read source WAV {}: {err}", path.display()))?;
    read_wav_playback_samples(&bytes).map(Arc::from)
}

fn read_interleaved_f32_file(
    cache_file: &PersistedPlaybackCacheFile,
) -> Result<Arc<[f32]>, String> {
    let bytes = fs::read(&cache_file.path).map_err(|err| {
        format!(
            "failed to read playback cache {}: {err}",
            cache_file.path.display()
        )
    })?;
    let expected_bytes = cache_file
        .sample_count
        .checked_mul(std::mem::size_of::<f32>() as u64)
        .ok_or_else(|| String::from("Playback cache is too large"))?;
    if bytes.len() as u64 != expected_bytes {
        return Err(String::from(
            "Playback cache size does not match its metadata",
        ));
    }
    let mut samples = Vec::with_capacity(cache_file.sample_count as usize);
    for chunk in bytes.chunks_exact(std::mem::size_of::<f32>()) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(Arc::from(samples))
}
