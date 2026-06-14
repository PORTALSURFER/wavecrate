use super::entry::CACHE_VERSION;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::app_dirs;
use crate::sample_sources::SourceId;
use std::path::{Path, PathBuf};

/// Extension used for persistent waveform cache payload files.
pub(super) const CACHE_FILE_EXTENSION: &str = "bin";

/// Resolve the root directory that stores all persistent waveform cache entries.
fn cache_root_dir() -> Result<PathBuf, String> {
    app_dirs::waveform_cache_dir().map_err(|err| format!("Failed to resolve waveform cache: {err}"))
}

/// Resolve the hashed directory for one source/path pair.
pub(super) fn cache_subdir(source_id: &SourceId, relative_path: &Path) -> Result<PathBuf, String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source_id.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(relative_path.to_string_lossy().as_bytes());
    Ok(cache_root_dir()?.join(hasher.finalize().to_hex()))
}

/// Build the cache file path for one source/path pair and file metadata snapshot.
pub(super) fn cache_file_path(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: FileMetadata,
) -> Result<PathBuf, String> {
    let file_name = format!(
        "v{CACHE_VERSION}_{}_{}.{}",
        metadata.file_size, metadata.modified_ns, CACHE_FILE_EXTENSION
    );
    Ok(cache_subdir(source_id, relative_path)?.join(file_name))
}
