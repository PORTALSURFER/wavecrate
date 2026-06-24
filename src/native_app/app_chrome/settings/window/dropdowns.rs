use radiant::prelude as ui;

mod projection;

use self::projection::{
    AudioDropdownProjection, audio_host_dropdown_projection, audio_output_dropdown_projection,
    audio_sample_rate_dropdown_projection, open_audio_settings_dropdown_projection,
};
use super::{
    AUDIO_SETTINGS_DROPDOWN_GAP, AUDIO_SETTINGS_LABELED_ROW_HEIGHT, AUDIO_SETTINGS_PANEL_PADDING,
    AUDIO_SETTINGS_ROW_SPACING, SETTINGS_CONTENT_WIDTH, SETTINGS_CONTENT_X,
};
use crate::native_app::app::{AudioSettingsDropdown, GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn audio_host_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let projection = audio_host_dropdown_projection(snapshot);
    ui::dropdown_trigger(projection.selected_label, projection.open)
        .toggle_message(GuiMessage::Settings(
            SettingsMessage::ToggleAudioBackendDropdown,
        ))
        .build()
}

pub(super) fn audio_output_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let projection = audio_output_dropdown_projection(snapshot);
    ui::dropdown_trigger(projection.selected_label, projection.open)
        .toggle_message(GuiMessage::Settings(
            SettingsMessage::ToggleAudioOutputDropdown,
        ))
        .build()
}

pub(super) fn audio_sample_rate_dropdown(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let projection = audio_sample_rate_dropdown_projection(snapshot);
    ui::dropdown_trigger(projection.selected_label, projection.open)
        .toggle_message(GuiMessage::Settings(
            SettingsMessage::ToggleAudioSampleRateDropdown,
        ))
        .build()
}

pub(super) fn audio_settings_dropdown_overlay(
    snapshot: &AudioSettingsSnapshot,
) -> ui::View<GuiMessage> {
    let Some((row_index, options)) = audio_settings_open_dropdown_options(snapshot) else {
        return ui::empty().fill_width();
    };
    ui::dropdown_menu_overlay_below_stacked_labeled_control(
        SETTINGS_CONTENT_X,
        AUDIO_SETTINGS_PANEL_PADDING,
        audio_settings_dropdown_cursor(snapshot, row_index),
        AUDIO_SETTINGS_DROPDOWN_GAP,
        Some(SETTINGS_CONTENT_WIDTH),
        options,
    )
}

fn audio_settings_open_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Option<(usize, Vec<ui::DropdownOption<GuiMessage>>)> {
    let projection = open_audio_settings_dropdown_projection(snapshot)?;
    let options = match projection.dropdown {
        AudioSettingsDropdown::Backend => audio_host_dropdown_options(snapshot),
        AudioSettingsDropdown::Output => audio_output_dropdown_options(snapshot),
        AudioSettingsDropdown::SampleRate => audio_sample_rate_dropdown_options(snapshot),
    };
    Some((projection.row_index, options))
}

fn audio_settings_dropdown_cursor(
    snapshot: &AudioSettingsSnapshot,
    labeled_rows_before: usize,
) -> ui::StackedLayoutCursor {
    ui::StackedLayoutCursor::new()
        .advanced(20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced_if(snapshot.error.is_some(), 20.0, AUDIO_SETTINGS_ROW_SPACING)
        .advanced_many(
            labeled_rows_before,
            AUDIO_SETTINGS_LABELED_ROW_HEIGHT,
            AUDIO_SETTINGS_ROW_SPACING,
        )
}

fn audio_host_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    audio_dropdown_options(audio_host_dropdown_projection(snapshot), |host| {
        SettingsMessage::SetAudioOutputHost(host)
    })
}

fn audio_output_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    audio_dropdown_options(audio_output_dropdown_projection(snapshot), |device| {
        SettingsMessage::SetAudioOutputDevice(device)
    })
}

fn audio_sample_rate_dropdown_options(
    snapshot: &AudioSettingsSnapshot,
) -> Vec<ui::DropdownOption<GuiMessage>> {
    audio_dropdown_options(
        audio_sample_rate_dropdown_projection(snapshot),
        |sample_rate| SettingsMessage::SetAudioOutputSampleRate(sample_rate),
    )
}

/// Convert an audio dropdown projection into Radiant menu options.
fn audio_dropdown_options<Value>(
    projection: AudioDropdownProjection<Value>,
    message: impl Fn(Option<Value>) -> SettingsMessage,
) -> Vec<ui::DropdownOption<GuiMessage>>
where
    Value: PartialEq,
{
    let selected = projection.selected_value;
    projection
        .options
        .into_iter()
        .map(|option| {
            ui::DropdownOption::for_optional_value(
                option.label,
                option.value,
                selected.as_ref(),
                |value| GuiMessage::Settings(message(value)),
            )
        })
        .collect()
}
