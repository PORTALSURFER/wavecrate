use super::super::super::projection_key_encoding::{
    encode_focus_context, encode_update_status, normalized_f32_to_milli,
};
use super::super::NonSegmentStaticProjectionCacheKey;
use super::shared::{hash_path_for_projection_key, hash_string_for_projection_key};
use crate::app_core::controller::AppController;
use crate::app_core::state::InlineFolderEditKind;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Build a projection key for static fields outside explicit segment keys.
pub(super) fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    let folder_inline = controller.ui.sources.folders.inline_edit.as_ref();
    let audio_engine = crate::app_core::native_shell::project_audio_engine_model(&controller.ui);
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        folder_inline_kind: match folder_inline.map(|draft| &draft.kind) {
            None => 0,
            Some(InlineFolderEditKind::Create { .. }) => 1,
            Some(InlineFolderEditKind::Rename { .. }) => 2,
        },
        folder_inline_path_hash: folder_inline.map(|draft| match &draft.kind {
            InlineFolderEditKind::Create { parent } => {
                hash_path_for_projection_key(parent.as_path())
            }
            InlineFolderEditKind::Rename { target } => {
                hash_path_for_projection_key(target.as_path())
            }
        }),
        folder_inline_name_hash: folder_inline
            .map(|draft| hash_string_for_projection_key(draft.name.as_str())),
        folder_inline_focus_requested: folder_inline.is_some_and(|draft| draft.focus_requested),
        folder_inline_select_all_on_focus: folder_inline
            .is_some_and(|draft| draft.select_all_on_focus_requested),
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
        focus_context: encode_focus_context(controller.ui.focus.context),
        trash_count: controller.ui.browser.trash.len(),
        neutral_count: controller.ui.browser.neutral.len(),
        keep_count: controller.ui.browser.keep.len(),
        audio_engine_chip_state: encode_audio_engine_chip_state(audio_engine.chip_state),
        audio_engine_chip_label_hash: hash_string_for_projection_key(&audio_engine.chip_label),
        audio_engine_detail_hash: audio_engine
            .detail_label
            .as_deref()
            .map(hash_string_for_projection_key),
        audio_engine_active_picker: encode_audio_picker_target(audio_engine.active_picker),
        audio_engine_output_host_hash: hash_string_for_projection_key(
            &audio_engine.output_host.value_label,
        ),
        audio_engine_output_device_hash: hash_string_for_projection_key(
            &audio_engine.output_device.value_label,
        ),
        audio_engine_output_sample_rate_hash: hash_string_for_projection_key(
            &audio_engine.output_sample_rate.value_label,
        ),
        audio_engine_input_host_hash: hash_string_for_projection_key(
            &audio_engine.input_host.value_label,
        ),
        audio_engine_input_device_hash: hash_string_for_projection_key(
            &audio_engine.input_device.value_label,
        ),
        audio_engine_input_sample_rate_hash: hash_string_for_projection_key(
            &audio_engine.input_sample_rate.value_label,
        ),
        audio_engine_output_host_options_hash: hash_audio_option_items(
            &audio_engine.output_host_options,
        ),
        audio_engine_output_device_options_hash: hash_audio_option_items(
            &audio_engine.output_device_options,
        ),
        audio_engine_output_sample_rate_options_hash: hash_audio_option_items(
            &audio_engine.output_sample_rate_options,
        ),
        audio_engine_input_host_options_hash: hash_audio_option_items(
            &audio_engine.input_host_options,
        ),
        audio_engine_input_device_options_hash: hash_audio_option_items(
            &audio_engine.input_device_options,
        ),
        audio_engine_input_sample_rate_options_hash: hash_audio_option_items(
            &audio_engine.input_sample_rate_options,
        ),
    }
}

fn encode_audio_engine_chip_state(
    state: crate::app_core::actions::NativeAudioEngineChipStateModel,
) -> u8 {
    match state {
        crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy => 0,
        crate::app_core::actions::NativeAudioEngineChipStateModel::Error => 1,
    }
}

fn encode_audio_picker_target(
    target: Option<crate::app_core::actions::NativeAudioPickerTargetModel>,
) -> u8 {
    match target {
        None => 0,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost) => 1,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputDevice) => 2,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate) => 3,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputHost) => 4,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputDevice) => 5,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputSampleRate) => 6,
    }
}

fn hash_audio_option_items(items: &[crate::app_core::actions::NativeAudioOptionItemModel]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for item in items {
        item.label.hash(&mut hasher);
        item.selected.hash(&mut hasher);
        match &item.value {
            crate::app_core::actions::NativeAudioOptionValueModel::OutputHost(value) => {
                0u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::OutputDevice(value) => {
                1u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::OutputSampleRate(value) => {
                2u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputHost(value) => {
                3u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputDevice(value) => {
                4u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputSampleRate(value) => {
                5u8.hash(&mut hasher);
                value.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}
