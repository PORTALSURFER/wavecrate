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

/// UI-projection options panel visibility state.
#[derive(Clone, Debug)]
pub struct OptionsPanelState {
    /// Whether the options panel is currently visible.
    pub open: bool,
    /// Current default identifier displayed in the options overview.
    pub default_identifier: String,
    /// Currently expanded audio picker, or `None` for the overview.
    pub active_audio_picker: Option<AudioPickerTarget>,
    /// Optional prompt state owned by the options panel.
    pub pending_prompt: Option<OptionsPanelPrompt>,
}

/// Prompt state owned by the ui-projection options panel.
#[derive(Clone, Debug)]
pub enum OptionsPanelPrompt {
    /// Edit the default identifier used by auto rename.
    DefaultIdentifier {
        /// Editable identifier value shown in the prompt input.
        value: String,
    },
}

impl Default for OptionsPanelState {
    fn default() -> Self {
        Self {
            open: false,
            default_identifier: String::from("portal"),
            active_audio_picker: None,
            pending_prompt: None,
        }
    }
}
