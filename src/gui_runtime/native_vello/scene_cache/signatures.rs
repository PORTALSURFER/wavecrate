//! Stable signature builders used to decide whether retained scenes can be reused.

use super::*;
use crate::gui::{
    fingerprint::StableFingerprint,
    focus::FocusSurface,
    list::EditableRowKind,
    native_shell::{FocusOverlayFingerprint, HoverOverlayFingerprint, WaveformToolbarHoverHint},
    panel::SplitPaneSlot,
    range::NormalizedRange,
};

fn fingerprint_mix_range(state: &mut StableFingerprint, range: &NormalizedRange) {
    state.mix_u16(range.start_milli);
    state.mix_u16(range.end_milli);
    state.mix_u32(range.start_micros);
    state.mix_u32(range.end_micros);
}

pub(in super::super) fn state_overlay_model_signature(model: &AppModel) -> u64 {
    let mut state = StableFingerprint::new();
    state.mix_usize(model.selected_column);
    state.mix_option_usize(model.browser.selected_visible_row);
    state.mix_option_usize(model.browser.anchor_visible_row);
    state.mix_option_usize(model.sources.selected_row);
    state.mix_option_usize(model.sources.focused_tree_row);
    state.mix_bool(model.confirm_prompt.visible);
    state.mix_u8(match model.confirm_prompt.kind {
        None => 0,
        Some(crate::compat_app_contract::ConfirmPromptKind::DestructiveOperation) => 1,
        Some(crate::compat_app_contract::ConfirmPromptKind::RenameContent) => 2,
        Some(crate::compat_app_contract::ConfirmPromptKind::RenameNavigationItem) => 3,
        Some(crate::compat_app_contract::ConfirmPromptKind::CreateNavigationItem) => 4,
        Some(crate::compat_app_contract::ConfirmPromptKind::RestoreRetainedItems) => 5,
        Some(crate::compat_app_contract::ConfirmPromptKind::PurgeRetainedItems) => 6,
        Some(crate::compat_app_contract::ConfirmPromptKind::EditConfiguration) => 7,
    });
    state.mix_str(&model.confirm_prompt.title);
    state.mix_str(&model.confirm_prompt.message);
    state.mix_str(&model.confirm_prompt.confirm_label);
    state.mix_str(&model.confirm_prompt.cancel_label);
    state.mix_option_str(model.confirm_prompt.target_label.as_deref());
    state.mix_option_str(model.confirm_prompt.input_value.as_deref());
    state.mix_option_str(model.confirm_prompt.input_placeholder.as_deref());
    state.mix_option_str(model.confirm_prompt.input_error.as_deref());
    state.mix_bool(model.progress_overlay.visible);
    state.mix_bool(model.progress_overlay.modal);
    state.mix_str(&model.progress_overlay.title);
    state.mix_option_str(model.progress_overlay.detail.as_deref());
    state.mix_usize(model.progress_overlay.completed);
    state.mix_usize(model.progress_overlay.total);
    state.mix_bool(model.progress_overlay.cancelable);
    state.mix_bool(model.progress_overlay.cancel_requested);
    state.mix_bool(model.drag_overlay.active);
    state.mix_str(&model.drag_overlay.label);
    state.mix_str(&model.drag_overlay.target_label);
    state.mix_bool(model.drag_overlay.valid_target);
    state.mix_option_u16(model.drag_overlay.pointer_x);
    state.mix_option_u16(model.drag_overlay.pointer_y);
    state.mix_u8(match model.update.status {
        crate::compat_app_contract::UpdateStatusModel::Idle => 0,
        crate::compat_app_contract::UpdateStatusModel::Checking => 1,
        crate::compat_app_contract::UpdateStatusModel::Available => 2,
        crate::compat_app_contract::UpdateStatusModel::Error => 3,
    });
    state.mix_bool(model.map.active);
    state.finish()
}

pub(in super::super) fn hover_overlay_model_signature(
    model: &AppModel,
    shell: &HoverOverlayFingerprint,
) -> u64 {
    let mut state = StableFingerprint::new();
    if let Some(hint) = shell.hovered_waveform_toolbar_hint {
        state.mix_bool(true);
        state.mix_u8(hint as u8);
        match hint {
            WaveformToolbarHoverHint::ChannelView => {
                state.mix_u8(match model.waveform_chrome.channel_view {
                    crate::gui::visualization::ChannelViewMode::Mono => 0,
                    crate::gui::visualization::ChannelViewMode::Stereo => 1,
                });
            }
            WaveformToolbarHoverHint::NormalizedAudition => {
                state.mix_bool(model.waveform_chrome.normalized_audition_enabled);
            }
            WaveformToolbarHoverHint::BpmValue => {
                state.mix_option_str(model.waveform.tempo_label.as_deref())
            }
            WaveformToolbarHoverHint::BpmSnap => {
                state.mix_bool(model.waveform_chrome.bpm_snap_enabled)
            }
            WaveformToolbarHoverHint::RelativeBpmGrid => {
                state.mix_bool(model.waveform_chrome.relative_bpm_grid_enabled)
            }
            WaveformToolbarHoverHint::TransientSnap => {
                state.mix_bool(model.waveform_chrome.transient_snap_enabled)
            }
            WaveformToolbarHoverHint::ShowTransients => {
                state.mix_bool(model.waveform_chrome.transient_markers_enabled)
            }
            WaveformToolbarHoverHint::SliceMode => {
                state.mix_bool(model.waveform_chrome.slice_mode_enabled)
            }
            WaveformToolbarHoverHint::Loop => {
                state.mix_bool(model.waveform_chrome.loop_lock_enabled);
                state.mix_bool(model.waveform.loop_enabled);
            }
            WaveformToolbarHoverHint::Compare => {
                state.mix_option_str(model.waveform_chrome.compare_anchor_label.as_deref());
            }
            WaveformToolbarHoverHint::Play => state.mix_bool(model.transport_running),
            WaveformToolbarHoverHint::SilenceSplit
            | WaveformToolbarHoverHint::ExactDedupe
            | WaveformToolbarHoverHint::CleanDuplicates
            | WaveformToolbarHoverHint::Stop
            | WaveformToolbarHoverHint::Record => {}
        }
    } else {
        state.mix_bool(false);
    }
    if let Some((pane, row_index)) = shell
        .hovered_folder_pane
        .zip(shell.hovered_folder_row_index)
    {
        state.mix_bool(true);
        state.mix_bool(model.drag_overlay.active);
        state.mix_bool(model.drag_overlay.valid_target);
        if let Some(row) = model.sources.folder_pane(pane).tree_rows.get(row_index) {
            state.mix_u8(match row.kind {
                EditableRowKind::CreateDraft => 0,
                EditableRowKind::RenameDraft => 1,
                EditableRowKind::Existing => 2,
            });
        } else {
            state.mix_u8(u8::MAX);
        }
    } else {
        state.mix_bool(false);
    }
    if shell.folder_create_editor_signature != 0 {
        state.mix_bool(true);
        state.mix_u8(match model.sources.active_folder_pane {
            SplitPaneSlot::Upper => 0,
            SplitPaneSlot::Lower => 1,
        });
        let active_tree_rows = &model.sources.active_folder_pane_model().tree_rows;
        let draft_row = active_tree_rows
            .iter()
            .find(|row| row.kind == EditableRowKind::RenameDraft)
            .or_else(|| {
                active_tree_rows
                    .iter()
                    .find(|row| row.kind == EditableRowKind::CreateDraft)
            });
        if let Some(row) = draft_row {
            state.mix_bool(true);
            state.mix_u8(match row.kind {
                EditableRowKind::CreateDraft => 0,
                EditableRowKind::RenameDraft => 1,
                EditableRowKind::Existing => 2,
            });
            state.mix_option_str(row.input_error.as_deref());
        } else {
            state.mix_bool(false);
        }
    } else {
        state.mix_bool(false);
    }
    state.finish()
}

pub(in super::super) fn focus_overlay_model_signature(
    model: &AppModel,
    shell: &FocusOverlayFingerprint,
) -> u64 {
    if !shell.has_focus_emphasis {
        return 0;
    }
    let mut state = StableFingerprint::new();
    state.mix_bool(model.browser.similarity_filtered);
    state.mix_bool(model.browser.duplicate_cleanup_active);
    state.mix_u8(match model.focus_context {
        FocusSurface::None => 0,
        FocusSurface::NavigationList => 1,
        FocusSurface::NavigationTree => 2,
        FocusSurface::ContentList => 3,
        FocusSurface::Timeline => 4,
    });
    for row in model
        .browser
        .rows
        .iter()
        .filter(|row| row.selected || row.focused)
    {
        state.mix_usize(row.visible_row);
        state.mix_bool(row.selected);
        state.mix_bool(row.focused);
        state.mix_bool(row.locked);
        state.mix_u8(row.playback_age_bucket as u8);
        if row.focused {
            state.mix_bool(row.missing);
            state.mix_option_i8(Some(row.rating_level));
            state.mix_str(&row.label);
            state.mix_option_str(row.bucket_label.as_deref());
        }
    }
    for (index, row) in model.sources.rows.iter().enumerate() {
        if row.assigned_to_upper_pane || row.assigned_to_lower_pane {
            state.mix_usize(index);
            state.mix_bool(row.assigned_to_upper_pane);
            state.mix_bool(row.assigned_to_lower_pane);
        }
    }
    for pane in [
        &model.sources.upper_folder_pane,
        &model.sources.lower_folder_pane,
    ] {
        for (index, row) in pane
            .tree_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.selected || row.focused)
        {
            state.mix_usize(index);
            state.mix_bool(row.selected);
            state.mix_bool(row.focused);
            if row.focused {
                state.mix_str(&row.label);
                state.mix_usize(row.depth);
            }
        }
    }
    state.finish()
}

pub(in super::super) fn modal_overlay_model_signature(model: &AppModel) -> u64 {
    let mut state = StableFingerprint::new();
    let preferences = model.options_panel.preference_state();
    state.mix_bool(preferences.visible);
    state.mix_str(&preferences.primary_text_value);
    for enabled in preferences.toggles {
        state.mix_bool(enabled);
    }
    state.mix_option_str(preferences.auxiliary_label.as_deref());
    state.mix_str(&model.browser_chrome.items_tab_label);
    state.mix_str(&model.browser_chrome.map_tab_label);
    state.mix_usize(
        model
            .columns
            .get(1)
            .map(|column| column.item_count)
            .unwrap_or(0),
    );
    state.mix_u64(state_overlay_model_signature(model));
    state.finish()
}

pub(in super::super) fn waveform_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = StableFingerprint::new();
    state.mix_bool(model.transport_running);
    if let Some(selection) = model.waveform_selection_milli {
        state.mix_bool(true);
        state.mix_u16(selection.start_milli);
        state.mix_u16(selection.end_milli);
        state.mix_u32(selection.start_micros);
        state.mix_u32(selection.end_micros);
    } else {
        state.mix_bool(false);
    }
    if let Some(edit_selection) = model.waveform_edit_selection_milli {
        state.mix_bool(true);
        fingerprint_mix_range(&mut state, &edit_selection);
    } else {
        state.mix_bool(false);
    }
    state.mix_usize(model.waveform_slices.len());
    for slice in &model.waveform_slices {
        fingerprint_mix_range(&mut state, &slice.range);
        state.mix_bool(slice.selected);
    }
    state.mix_option_u16(model.waveform_edit_fade_in_end_milli);
    state.mix_option_u32(model.waveform_edit_fade_in_end_micros);
    state.mix_option_u16(model.waveform_edit_fade_in_mute_start_milli);
    state.mix_option_u32(model.waveform_edit_fade_in_mute_start_micros);
    state.mix_option_u16(model.waveform_edit_fade_in_curve_milli);
    state.mix_option_u16(model.waveform_edit_fade_out_start_milli);
    state.mix_option_u32(model.waveform_edit_fade_out_start_micros);
    state.mix_option_u16(model.waveform_edit_fade_out_mute_end_milli);
    state.mix_option_u32(model.waveform_edit_fade_out_mute_end_micros);
    state.mix_option_u16(model.waveform_edit_fade_out_curve_milli);
    state.mix_bool(model.waveform_loop_enabled);
    state.mix_bool(model.waveform_loop_lock_enabled);
    state.mix_option_u16(model.waveform_cursor_milli);
    state.mix_option_u16(model.waveform_playhead_milli);
    state.mix_option_u32(model.waveform_playhead_micros);
    state.mix_u16(model.waveform_view_start_milli);
    state.mix_u16(model.waveform_view_end_milli);
    state.mix_u32(model.waveform_view_start_micros);
    state.mix_u32(model.waveform_view_end_micros);
    state.mix_bool(model.waveform_loading);
    if let Some(signature) = model.waveform_image_signature {
        state.mix_bool(true);
        state.mix_u64(signature);
    } else {
        state.mix_bool(false);
    }
    state.finish()
}

pub(in super::super) fn chrome_motion_overlay_model_signature(model: &NativeMotionModel) -> u64 {
    let mut state = StableFingerprint::new();
    state.mix_bool(model.transport_running);
    state.mix_bool(model.map_active);
    state.mix_option_str(model.waveform_tempo_label.as_deref());
    state.mix_option_str(model.waveform_zoom_label.as_deref());
    state.mix_option_str(model.waveform_loaded_label.as_deref());
    state.mix_u8(match model.waveform_channel_view {
        crate::gui::visualization::ChannelViewMode::Mono => 0,
        crate::gui::visualization::ChannelViewMode::Stereo => 1,
    });
    state.mix_bool(model.waveform_normalized_audition_enabled);
    state.mix_bool(model.waveform_bpm_snap_enabled);
    state.mix_bool(model.waveform_relative_bpm_grid_enabled);
    state.mix_bool(model.waveform_transient_snap_enabled);
    state.mix_bool(model.waveform_transient_markers_enabled);
    state.mix_bool(model.waveform_slice_mode_enabled);
    state.mix_bool(model.waveform_exact_duplicate_cleanup_available);
    state.mix_bool(model.waveform_loop_enabled);
    state.mix_bool(model.waveform_loop_lock_enabled);
    state.mix_str(&model.waveform_transport_hint);
    state.mix_str(&model.status_right);
    state.finish()
}

pub(in super::super) fn static_segment_style_signature(style: &StyleTokens) -> u64 {
    let mut state = StableFingerprint::new();
    state.mix_rgba8(style.clear_color);
    state.mix_rgba8(style.surface_base);
    state.mix_rgba8(style.surface_raised);
    state.mix_rgba8(style.surface_overlay);
    state.mix_rgba8(style.border);
    state.mix_rgba8(style.border_emphasis);
    state.mix_f32(style.sizing.border_width);
    state.mix_f32(style.sizing.focus_stroke_width);
    state.mix_f32(style.sizing.font_header);
    state.mix_f32(style.sizing.font_body);
    state.mix_f32(style.sizing.font_meta);
    state.mix_f32(style.sizing.font_status);
    state.finish()
}
