use super::super::super::projection_key_encoding::{
    encode_focus_context, encode_update_status, normalized_f32_to_milli,
};
use super::super::{NonSegmentOverlayProjectionCacheKey, NonSegmentStaticProjectionCacheKey};
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
    let audio_chip = crate::app_core::native_shell::project_audio_engine_chip_model(&controller.ui);
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
        audio_engine_chip_state: encode_audio_engine_chip_state(audio_chip.chip_state),
        audio_engine_chip_label_hash: hash_string_for_projection_key(&audio_chip.chip_label),
    }
}

/// Build a projection key for overlay fields outside explicit static segment buckets.
pub(super) fn build_non_segment_overlay_projection_key(
    controller: &AppController,
) -> NonSegmentOverlayProjectionCacheKey {
    let options_panel = crate::app_core::native_shell::project_options_panel_model(&controller.ui);
    let progress_overlay =
        crate::app_core::native_shell::project_progress_overlay_model(&controller.ui);
    let confirm_prompt =
        crate::app_core::native_shell::project_confirm_prompt_model(&controller.ui);
    let drag_overlay = crate::app_core::native_shell::project_drag_overlay_model(&controller.ui);
    let audio_engine_overlay_hash = if options_panel.visible {
        hash_audio_engine_model(&crate::app_core::native_shell::project_audio_engine_model(
            &controller.ui,
        ))
    } else {
        0
    };
    NonSegmentOverlayProjectionCacheKey {
        audio_engine_overlay_hash,
        options_panel_hash: hash_options_panel_model(&options_panel),
        progress_overlay_hash: hash_progress_overlay_model(&progress_overlay),
        confirm_prompt_hash: hash_confirm_prompt_model(&confirm_prompt),
        drag_overlay_hash: hash_drag_overlay_model(&drag_overlay),
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

fn hash_audio_engine_model(model: &crate::app_core::actions::NativeAudioEngineModel) -> u64 {
    let mut hasher = DefaultHasher::new();
    encode_audio_engine_chip_state(model.chip_state).hash(&mut hasher);
    model.chip_label.hash(&mut hasher);
    model.detail_label.hash(&mut hasher);
    model.output_host.label.hash(&mut hasher);
    model.output_host.value_label.hash(&mut hasher);
    model.output_device.label.hash(&mut hasher);
    model.output_device.value_label.hash(&mut hasher);
    model.output_sample_rate.label.hash(&mut hasher);
    model.output_sample_rate.value_label.hash(&mut hasher);
    model.input_host.label.hash(&mut hasher);
    model.input_host.value_label.hash(&mut hasher);
    model.input_device.label.hash(&mut hasher);
    model.input_device.value_label.hash(&mut hasher);
    model.input_sample_rate.label.hash(&mut hasher);
    model.input_sample_rate.value_label.hash(&mut hasher);
    match model.active_picker {
        None => 0u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost) => 1u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputDevice) => 2u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate) => 3u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputHost) => 4u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputDevice) => 5u8,
        Some(crate::app_core::actions::NativeAudioPickerTargetModel::InputSampleRate) => 6u8,
    }
    .hash(&mut hasher);
    hash_audio_option_items(&model.output_host_options, &mut hasher);
    hash_audio_option_items(&model.output_device_options, &mut hasher);
    hash_audio_option_items(&model.output_sample_rate_options, &mut hasher);
    hash_audio_option_items(&model.input_host_options, &mut hasher);
    hash_audio_option_items(&model.input_device_options, &mut hasher);
    hash_audio_option_items(&model.input_sample_rate_options, &mut hasher);
    hasher.finish()
}

fn hash_audio_option_items(
    items: &[crate::app_core::actions::NativeAudioOptionItemModel],
    hasher: &mut DefaultHasher,
) {
    items.len().hash(hasher);
    for item in items {
        item.label.hash(hasher);
        item.selected.hash(hasher);
        match &item.value {
            crate::app_core::actions::NativeAudioOptionValueModel::OutputHost(value) => {
                0u8.hash(hasher);
                value.hash(hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::OutputDevice(value) => {
                1u8.hash(hasher);
                value.hash(hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::OutputSampleRate(value) => {
                2u8.hash(hasher);
                value.hash(hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputHost(value) => {
                3u8.hash(hasher);
                value.hash(hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputDevice(value) => {
                4u8.hash(hasher);
                value.hash(hasher);
            }
            crate::app_core::actions::NativeAudioOptionValueModel::InputSampleRate(value) => {
                5u8.hash(hasher);
                value.hash(hasher);
            }
        }
    }
}

fn hash_options_panel_model(model: &crate::app_core::actions::NativeOptionsPanelModel) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.visible.hash(&mut hasher);
    model.input_monitoring_enabled.hash(&mut hasher);
    model.advance_after_rating_enabled.hash(&mut hasher);
    model.destructive_yolo_mode_enabled.hash(&mut hasher);
    model.invert_waveform_scroll_enabled.hash(&mut hasher);
    model.trash_folder_label.hash(&mut hasher);
    hasher.finish()
}

fn hash_progress_overlay_model(
    model: &crate::app_core::actions::NativeProgressOverlayModel,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.visible.hash(&mut hasher);
    model.modal.hash(&mut hasher);
    model.title.hash(&mut hasher);
    model.detail.hash(&mut hasher);
    model.completed.hash(&mut hasher);
    model.total.hash(&mut hasher);
    model.cancelable.hash(&mut hasher);
    model.cancel_requested.hash(&mut hasher);
    hasher.finish()
}

fn hash_confirm_prompt_model(model: &crate::app_core::actions::NativeConfirmPromptModel) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.visible.hash(&mut hasher);
    match model.kind {
        None => 0u8,
        Some(crate::app_core::actions::NativeConfirmPromptKind::DestructiveEdit) => 1u8,
        Some(crate::app_core::actions::NativeConfirmPromptKind::BrowserRename) => 2u8,
        Some(crate::app_core::actions::NativeConfirmPromptKind::FolderRename) => 3u8,
        Some(crate::app_core::actions::NativeConfirmPromptKind::FolderCreate) => 4u8,
        Some(crate::app_core::actions::NativeConfirmPromptKind::RestoreRetainedFolderDeletes) => {
            5u8
        }
        Some(crate::app_core::actions::NativeConfirmPromptKind::PurgeRetainedFolderDeletes) => 6u8,
    }
    .hash(&mut hasher);
    model.title.hash(&mut hasher);
    model.message.hash(&mut hasher);
    model.confirm_label.hash(&mut hasher);
    model.cancel_label.hash(&mut hasher);
    model.target_label.hash(&mut hasher);
    model.input_value.hash(&mut hasher);
    model.input_placeholder.hash(&mut hasher);
    model.input_error.hash(&mut hasher);
    hasher.finish()
}

fn hash_drag_overlay_model(model: &crate::app_core::actions::NativeDragOverlayModel) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.active.hash(&mut hasher);
    model.label.hash(&mut hasher);
    model.target_label.hash(&mut hasher);
    model.valid_target.hash(&mut hasher);
    model.pointer_x.hash(&mut hasher);
    model.pointer_y.hash(&mut hasher);
    hasher.finish()
}
