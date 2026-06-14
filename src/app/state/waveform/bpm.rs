pub(super) struct WaveformBpmState {
    pub(super) snap_enabled: bool,
    pub(super) relative_grid_enabled: bool,
    pub(super) lock_enabled: bool,
    pub(super) stretch_enabled: bool,
    pub(super) input: String,
    pub(super) value: Option<f32>,
}

impl Default for WaveformBpmState {
    fn default() -> Self {
        Self {
            snap_enabled: false,
            relative_grid_enabled: false,
            lock_enabled: false,
            stretch_enabled: false,
            input: "142".to_string(),
            value: Some(142.0),
        }
    }
}
