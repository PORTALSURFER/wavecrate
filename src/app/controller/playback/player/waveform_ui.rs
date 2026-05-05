use super::*;

/// Update the active playback selection and its cached duration label.
pub(crate) fn apply_selection(controller: &mut AppController, range: Option<SelectionRange>) {
    let label = range.and_then(|selection| selection_duration_label(controller, selection));
    if let Some(selection) = range {
        controller.ui.waveform.last_bpm_grid_origin = selection.start();
    }
    controller.ui.waveform.selection = range;
    controller.ui.waveform.selection_duration = label;
}

/// Update the edit selection and synchronize edit-fade preview state.
pub(crate) fn apply_edit_selection(controller: &mut AppController, range: Option<SelectionRange>) {
    let previous = controller.ui.waveform.edit_selection;
    controller.ui.waveform.edit_selection = range;
    if let Some(player) = controller.audio.player.as_ref() {
        player.borrow().set_edit_fade_state(range);
    }
    let had_effects = previous.is_some_and(|selection| selection.has_edit_effects());
    let has_effects = range.is_some_and(|selection| selection.has_edit_effects());
    if had_effects || has_effects {
        controller.refresh_waveform_image();
    }
}

/// Update the hover time indicator for the waveform.
pub(crate) fn update_waveform_hover_time(controller: &mut AppController, position: Option<f32>) {
    if let (Some(position), Some(audio)) =
        (position, controller.sample_view.wav.loaded_audio.as_ref())
    {
        let clamped = position.clamp(0.0, 1.0);
        let seconds = audio.duration_seconds * clamped;
        controller.ui.waveform.hover_time_label =
            Some(super::super::format_timestamp_hms_ms(seconds));
    } else {
        controller.ui.waveform.hover_time_label = None;
    }
}

/// Format the duration label for one waveform selection span.
pub(crate) fn selection_duration_label(
    controller: &AppController,
    range: SelectionRange,
) -> Option<String> {
    let audio = controller.sample_view.wav.loaded_audio.as_ref()?;
    let seconds = (audio.duration_seconds * range.width()).max(0.0);
    Some(super::super::format_selection_duration(seconds))
}

/// Apply output volume to runtime audio state without persisting configuration.
pub(crate) fn apply_volume(controller: &mut AppController, volume: f32) {
    let clamped = volume.clamp(0.0, 1.0);
    controller.ui.volume = clamped;
    if let Some(player) = controller.audio.player.as_ref() {
        player.borrow_mut().set_volume(clamped);
    }
}
