pub(in crate::native_app) use super::super::metadata::MetadataTagInputMode;
pub(in crate::native_app) use crate::native_app::audio::sample_load_actions::{
    KEYBOARD_SAMPLE_LOAD_DEBOUNCE, UNCACHED_SAMPLE_LOAD_DEBOUNCE,
};
pub(in crate::native_app) use crate::native_app::sample_library::file_actions::{
    format_copy_path, normalize_wav_file_in_place,
};
pub(in crate::native_app) use crate::native_app::waveform::{
    WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID,
};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::waveform_panel;
use radiant::prelude as ui;

pub(in crate::native_app) fn waveform_loading_visual(
    file_name: &str,
    progress: f32,
) -> ui::View<GuiMessage> {
    waveform_panel::waveform_loading_visual(file_name, progress)
}
