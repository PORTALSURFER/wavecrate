use super::PersistentWaveformHit;
use super::entry::{CACHE_VERSION, PersistentWaveformEntry};
use crate::waveform::DecodedWaveform;
use std::path::Path;
use std::sync::Arc;

pub(super) fn decode_entry(path: &Path, bytes: &[u8]) -> Option<PersistentWaveformEntry> {
    match bincode::deserialize(bytes) {
        Ok(entry) => Some(entry),
        Err(err) => {
            tracing::warn!(
                "Failed to decode persistent waveform cache {}: {err}",
                path.display()
            );
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

pub(super) fn entry_into_hit_if_current(
    path: &Path,
    entry: PersistentWaveformEntry,
) -> Option<PersistentWaveformHit> {
    if entry.version() != CACHE_VERSION {
        let _ = std::fs::remove_file(path);
        return None;
    }
    Some(entry.into_hit())
}

pub(super) fn encode_entry(
    relative_path: &Path,
    decoded: &Arc<DecodedWaveform>,
    transients: &Arc<[f32]>,
) -> Option<Vec<u8>> {
    let entry = PersistentWaveformEntry::from_runtime(decoded, transients);
    match bincode::serialize(&entry) {
        Ok(bytes) => Some(bytes),
        Err(err) => {
            tracing::warn!(
                "Failed to encode waveform cache entry {}: {err}",
                relative_path.display()
            );
            None
        }
    }
}
