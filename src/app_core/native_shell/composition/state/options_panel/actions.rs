//! Options-panel action definitions and paired-picker helpers.

use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

pub(super) fn audio_overview_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    let paired_device = model.paired_device_panel();
    vec![
        (
            format!(
                "{}: {}",
                paired_device.primary_group().label,
                paired_device.primary_group().value_label
            ),
            UiAction::OpenPrimaryGroupPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.primary_item().label,
                paired_device.primary_item().value_label
            ),
            UiAction::OpenPrimaryItemPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.primary_number().label,
                paired_device.primary_number().value_label
            ),
            UiAction::OpenPrimaryNumberPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_group().label,
                paired_device.secondary_group().value_label
            ),
            UiAction::OpenSecondaryGroupPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_item().label,
                paired_device.secondary_item().value_label
            ),
            UiAction::OpenSecondaryItemPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_number().label,
                paired_device.secondary_number().value_label
            ),
            UiAction::OpenSecondaryNumberPicker,
        ),
    ]
}

pub(super) fn legacy_options_panel_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    vec![
        (
            format!(
                "Auto Rename Identifier: {}",
                model.options_panel.default_identifier
            ),
            UiAction::EditDefaultIdentifier,
        ),
        (
            on_off_text(
                "Input Monitor",
                model.options_panel.input_monitoring_enabled,
            ),
            UiAction::SetInputMonitoringEnabled {
                enabled: !model.options_panel.input_monitoring_enabled,
            },
        ),
        (
            on_off_text(
                "Advance After Rating",
                model.options_panel.advance_after_rating_enabled,
            ),
            UiAction::SetAdvanceAfterRatingEnabled {
                enabled: !model.options_panel.advance_after_rating_enabled,
            },
        ),
        (
            on_off_text(
                "YOLO Edits",
                model.options_panel.destructive_yolo_mode_enabled,
            ),
            UiAction::SetDestructiveYoloMode {
                enabled: !model.options_panel.destructive_yolo_mode_enabled,
            },
        ),
        (
            on_off_text(
                "Invert Scroll",
                model.options_panel.invert_waveform_scroll_enabled,
            ),
            UiAction::SetInvertWaveformScroll {
                enabled: !model.options_panel.invert_waveform_scroll_enabled,
            },
        ),
        (
            format!(
                "Trash Folder: {}",
                model
                    .options_panel
                    .trash_folder_label
                    .as_deref()
                    .unwrap_or("Not set")
            ),
            UiAction::PickTrashFolder,
        ),
        (String::from("Open Trash Folder"), UiAction::OpenTrashFolder),
        (String::from("Close"), UiAction::CloseOptionsPanel),
    ]
}

pub(super) fn options_panel_title(model: &AppModel) -> String {
    model
        .paired_device_panel()
        .active_picker()
        .map(audio_picker_title)
        .unwrap_or_else(|| String::from("Audio Engine"))
}

pub(super) fn picker_options(
    model: &AppModel,
    target: native_model::PairedPickerTargetModel,
) -> &[native_model::PairedPickerOptionModel] {
    model.paired_device_panel().options_for(target)
}

/// Map one projected paired-picker option into the native action it emits.
pub(super) fn picker_action(value: &native_model::PairedPickerValueModel) -> UiAction {
    match value {
        native_model::PairedPickerValueModel::PrimaryGroup(group_id) => UiAction::SetPrimaryGroup {
            group_id: group_id.clone(),
        },
        native_model::PairedPickerValueModel::PrimaryItem(item_name) => UiAction::SetPrimaryItem {
            item_name: item_name.clone(),
        },
        native_model::PairedPickerValueModel::PrimaryNumber(value) => {
            UiAction::SetPrimaryNumber { value: *value }
        }
        native_model::PairedPickerValueModel::SecondaryGroup(group_id) => {
            UiAction::SetSecondaryGroup {
                group_id: group_id.clone(),
            }
        }
        native_model::PairedPickerValueModel::SecondaryItem(item_name) => {
            UiAction::SetSecondaryItem {
                item_name: item_name.clone(),
            }
        }
        native_model::PairedPickerValueModel::SecondaryNumber(value) => {
            UiAction::SetSecondaryNumber { value: *value }
        }
    }
}

/// Return the title text for the active audio picker target.
fn audio_picker_title(target: native_model::PairedPickerTargetModel) -> String {
    match target {
        native_model::PairedPickerTargetModel::PrimaryGroup => String::from("Output Host"),
        native_model::PairedPickerTargetModel::PrimaryItem => String::from("Output Device"),
        native_model::PairedPickerTargetModel::PrimaryNumber => String::from("Output Sample Rate"),
        native_model::PairedPickerTargetModel::SecondaryGroup => String::from("Input Host"),
        native_model::PairedPickerTargetModel::SecondaryItem => String::from("Input Device"),
        native_model::PairedPickerTargetModel::SecondaryNumber => String::from("Input Sample Rate"),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}
