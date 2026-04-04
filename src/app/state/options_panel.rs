/// Audio field currently expanded into a picker inside the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPickerTarget {
    /// Output host/backend picker.
    OutputHost,
    /// Output device picker.
    OutputDevice,
    /// Output sample-rate picker.
    OutputSampleRate,
    /// Input host/backend picker.
    InputHost,
    /// Input device picker.
    InputDevice,
    /// Input sample-rate picker.
    InputSampleRate,
}

/// Native-shell options panel visibility state.
#[derive(Clone, Debug, Default)]
pub struct OptionsPanelState {
    /// Whether the options panel is currently visible.
    pub open: bool,
    /// Currently expanded audio picker, or `None` for the overview.
    pub active_audio_picker: Option<AudioPickerTarget>,
}
