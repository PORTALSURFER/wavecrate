use super::*;
use crate::app::controller::playback::audio_cache::FileMetadata;
use std::fs;
use std::path::Path;

impl AppController {
    pub(crate) fn read_waveform_bytes(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<Vec<u8>, String> {
        let full_path = source.root.join(relative_path);
        let bytes = fs::read(&full_path)
            .map_err(|err| format!("Failed to read {}: {err}", full_path.display()))?;
        Ok(crate::wav_sanitize::sanitize_wav_bytes(bytes))
    }

    pub(crate) fn current_file_metadata(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<FileMetadata, String> {
        let full_path = source.root.join(relative_path);
        let metadata = fs::metadata(&full_path)
            .map_err(|err| format!("Failed to read {}: {err}", full_path.display()))?;
        let modified_ns = metadata
            .modified()
            .map_err(|err| format!("Missing modified time for {}: {err}", full_path.display()))?
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|_| "File modified time is before epoch".to_string())?
            .as_nanos() as i64;
        Ok(FileMetadata {
            file_size: metadata.len(),
            modified_ns,
        })
    }
}
