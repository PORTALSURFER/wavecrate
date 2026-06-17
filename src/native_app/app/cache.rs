use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::native_app::waveform::WaveformFile;

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformCacheEntry {
    pub(in crate::native_app) byte_len: usize,
    pub(in crate::native_app) file: Arc<WaveformFile>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformCacheWarmResult {
    pub(in crate::native_app) loaded: Vec<(PathBuf, Arc<WaveformFile>)>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct ActiveFolderCacheWarmResult {
    pub(in crate::native_app) folder_id: String,
    pub(in crate::native_app) loaded: Vec<(PathBuf, Arc<WaveformFile>)>,
    pub(in crate::native_app) processed: usize,
    pub(in crate::native_app) cancelled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct WaveformCacheIndicatorRefreshResult {
    pub(in crate::native_app) probed_paths: Vec<PathBuf>,
    pub(in crate::native_app) playback_ready_paths: HashSet<PathBuf>,
    pub(in crate::native_app) warm_candidate_paths: HashSet<PathBuf>,
}

impl PartialEq for WaveformCacheWarmResult {
    fn eq(&self, other: &Self) -> bool {
        self.loaded
            .iter()
            .map(|(path, _)| path)
            .eq(other.loaded.iter().map(|(path, _)| path))
    }
}

impl Eq for WaveformCacheWarmResult {}

impl PartialEq for ActiveFolderCacheWarmResult {
    fn eq(&self, other: &Self) -> bool {
        self.folder_id == other.folder_id
            && self.cancelled == other.cancelled
            && self.processed == other.processed
            && self
                .loaded
                .iter()
                .map(|(path, _)| path)
                .eq(other.loaded.iter().map(|(path, _)| path))
    }
}
