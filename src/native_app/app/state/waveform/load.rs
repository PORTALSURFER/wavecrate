use crate::native_app::app::SampleSelectionLoadState;

pub(in crate::native_app) struct WaveformLoadState {
    pub(in crate::native_app) progress: f32,
    pub(in crate::native_app) target_progress: f32,
    pub(in crate::native_app) label: Option<String>,
    pub(in crate::native_app) selection: SampleSelectionLoadState,
}

impl Default for WaveformLoadState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            target_progress: 0.0,
            label: None,
            selection: SampleSelectionLoadState::default(),
        }
    }
}
