use crate::app_core::state::UiState;

pub(super) fn audio_engine_detail_label(ui: &UiState, output_mismatch: bool) -> Option<String> {
    ui.audio
        .output_runtime_error
        .clone()
        .or_else(|| ui.audio.warning.clone())
        .or_else(|| {
            if output_mismatch {
                Some(String::from(
                    "Selected output differs from the active engine",
                ))
            } else {
                None
            }
        })
        .or_else(|| {
            if ui.audio.applied.is_none() {
                Some(String::from("Audio unavailable"))
            } else {
                None
            }
        })
}

pub(super) fn output_selection_mismatch(ui: &UiState) -> bool {
    let Some(applied) = ui.audio.applied.as_ref() else {
        return false;
    };
    ui.audio
        .selected
        .host
        .as_deref()
        .is_some_and(|host| host != applied.host_id)
        || ui
            .audio
            .selected
            .device
            .as_deref()
            .is_some_and(|device| device != applied.device_name)
        || ui
            .audio
            .selected
            .sample_rate
            .is_some_and(|rate| rate != applied.sample_rate)
}
