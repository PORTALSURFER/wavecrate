use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::format::CACHE_FORMAT_VERSION;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CacheIdentity {
    pub(super) file_len: u64,
    pub(super) modified_ns: u128,
}

impl CacheIdentity {
    pub(super) fn for_path(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path).map_err(|err| err.to_string())?;
        let modified_ns = metadata
            .modified()
            .map_err(|err| err.to_string())?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|err| err.to_string())?
            .as_nanos();
        Ok(Self {
            file_len: metadata.len(),
            modified_ns,
        })
    }
}

pub(super) fn cache_path_for_identity(
    path: &Path,
    identity: &CacheIdentity,
) -> Result<PathBuf, String> {
    cache_path_for_identity_with_version(path, identity, CACHE_FORMAT_VERSION)
}

pub(super) fn cache_path_for_identity_with_version(
    path: &Path,
    identity: &CacheIdentity,
    version: u32,
) -> Result<PathBuf, String> {
    let dir = wavecrate::app_dirs::waveform_cache_dir().map_err(|err| err.to_string())?;
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    path.to_string_lossy().hash(&mut hasher);
    identity.file_len.hash(&mut hasher);
    identity.modified_ns.hash(&mut hasher);
    Ok(dir.join(format!("{:016x}.wfc", hasher.finish())))
}

pub(super) fn playback_ready_marker_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("ready")
}

pub(super) fn playback_sidecar_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("pcm")
}

pub(super) fn playback_sidecar_valid(sidecar_path: &Path, sample_count: u64) -> bool {
    let expected_len = sample_count.saturating_mul(std::mem::size_of::<f32>() as u64);
    sample_count > 0
        && sidecar_path
            .metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == expected_len)
}
