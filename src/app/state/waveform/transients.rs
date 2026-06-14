use std::sync::Arc;

pub(super) struct WaveformTransientState {
    pub(super) positions: Arc<[f32]>,
    pub(super) markers_enabled: bool,
    pub(super) snap_enabled: bool,
    pub(super) cache_token: Option<u64>,
}

impl Default for WaveformTransientState {
    fn default() -> Self {
        Self {
            positions: Arc::from([]),
            markers_enabled: true,
            snap_enabled: false,
            cache_token: None,
        }
    }
}
