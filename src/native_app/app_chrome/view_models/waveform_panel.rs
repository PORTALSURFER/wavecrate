use crate::native_app::app::{NativeAppState, NativeFileDropHover};
use crate::native_app::waveform::WaveformState;

pub(in crate::native_app) struct WaveformPanelViewModel<'a> {
    pub(in crate::native_app) waveform: &'a WaveformState,
    pub(in crate::native_app) drop_hover: Option<&'a NativeFileDropHover>,
    pub(in crate::native_app) loading_label: Option<&'a str>,
    pub(in crate::native_app) block_input_while_loading: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
    pub(in crate::native_app) beat_guides_enabled: bool,
    pub(in crate::native_app) beat_guide_count: u8,
}

impl<'a> WaveformPanelViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a NativeAppState) -> Self {
        Self {
            waveform: &state.waveform.current,
            drop_hover: state.ui.browser_interaction.native_file_drop_hover.as_ref(),
            loading_label: state.waveform.load.label.as_deref(),
            block_input_while_loading: state.waveform_input_blocked_by_sample_load(),
            help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
            beat_guides_enabled: state.ui.chrome.beat_guides_enabled,
            beat_guide_count: state.ui.chrome.beat_guide_count,
        }
    }
}
