//! Disk-loading facade for waveform payloads.

use super::*;
use std::io::Read;
use std::path::Path;

const MAX_WAVEFORM_BYTES: u64 = 512 * 1024 * 1024;

impl WaveformRenderer {
    /// Load a wav file from disk and return its pixels, raw bytes, and duration.
    ///
    /// This enforces a 512 MB size cap to avoid loading large files into memory all at once.
    pub fn load_waveform(&self, path: &Path) -> Result<LoadedWaveform, WaveformLoadError> {
        let bytes = read_audio_bytes_with_limit(path, MAX_WAVEFORM_BYTES)?;
        let decoded = self.decode_from_bytes(&bytes)?;
        let image = self.render_color_image_for_mode(&decoded, WaveformChannelView::Mono);
        Ok(LoadedWaveform {
            image,
            audio_bytes: bytes,
            duration_seconds: decoded.duration_seconds,
        })
    }
}

fn read_audio_bytes_with_limit(path: &Path, max_bytes: u64) -> Result<Vec<u8>, WaveformLoadError> {
    let metadata = std::fs::metadata(path).map_err(|source| WaveformLoadError::Metadata {
        path: path.to_path_buf(),
        source,
    })?;
    let size = metadata.len();
    if size > max_bytes {
        return Err(WaveformLoadError::TooLarge {
            path: path.to_path_buf(),
            size_bytes: size,
            limit_bytes: max_bytes,
        });
    }

    let file = std::fs::File::open(path).map_err(|source| WaveformLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut limited = file.take(max_bytes + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|source| WaveformLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(WaveformLoadError::TooLarge {
            path: path.to_path_buf(),
            size_bytes: bytes.len() as u64,
            limit_bytes: max_bytes,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_audio_bytes_with_limit_rejects_files_over_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.wav");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(10).unwrap();

        let err = read_audio_bytes_with_limit(&path, 5).unwrap_err();
        assert!(matches!(err, WaveformLoadError::TooLarge { .. }));
    }
}
