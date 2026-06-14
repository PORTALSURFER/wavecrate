use super::super::super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::persistent_waveform_cache::persist_waveform_cache_entry;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

impl AppController {
    pub(super) fn cache_loaded_waveform_transients(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        metadata: crate::app::controller::playback::audio_cache::FileMetadata,
        decoded: &Arc<DecodedWaveform>,
        loaded_bytes: Arc<[u8]>,
        audio_path: Option<PathBuf>,
        transients: Arc<[f32]>,
        stretched: bool,
    ) {
        if stretched {
            return;
        }
        let key = CacheKey::new(source_id, relative_path);
        self.audio.cache.insert(
            key,
            metadata,
            Arc::clone(decoded),
            loaded_bytes,
            audio_path,
            transients.clone(),
        );
        persist_loaded_waveform_transients(source_id, relative_path, metadata, decoded, transients);
    }
}

fn persist_loaded_waveform_transients(
    source_id: &SourceId,
    relative_path: &Path,
    metadata: crate::app::controller::playback::audio_cache::FileMetadata,
    decoded: &Arc<DecodedWaveform>,
    transients: Arc<[f32]>,
) {
    let source_id = source_id.clone();
    let relative_path = relative_path.to_path_buf();
    let decoded = Arc::clone(decoded);
    thread::spawn(move || {
        persist_waveform_cache_entry(&source_id, &relative_path, metadata, &decoded, &transients);
    });
}
