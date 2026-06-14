use crate::native_app::app::NativeAppState;
use crate::native_app::app_chrome::view_models::{
    sample_browser::{SampleBrowserViewModel, SampleBrowserViewProjection},
    toolbar::MainToolbarViewModel,
    waveform_panel::WaveformPanelViewModel,
};

pub(in crate::native_app) struct SampleWorkspaceViewModel<'a> {
    pub(in crate::native_app) toolbar: MainToolbarViewModel,
    pub(in crate::native_app) waveform: WaveformPanelViewModel<'a>,
    pub(in crate::native_app) browser: SampleBrowserViewModel<'a>,
}

impl<'a> SampleWorkspaceViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a NativeAppState) -> Self {
        Self {
            toolbar: MainToolbarViewModel::from_app_state(state),
            waveform: WaveformPanelViewModel::from_app_state(state),
            browser: SampleBrowserViewModel::from_projection(
                SampleBrowserViewProjection::from_prepared_app_state(state),
            ),
        }
    }
}
