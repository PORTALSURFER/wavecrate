//! Waveform selection and drag routing for native actions.

use super::super::AppController;
use crate::app::state::FolderPaneId;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::{DragSource, DragTarget, UiPoint};
use radiant::app::FolderPaneIdModel;

/// Try to dispatch waveform selection and edit-selection native actions.
pub(super) fn apply_waveform_selection_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::BeginWaveformSelectionAt { anchor_micros } => {
            controller.start_selection_drag(normalize_waveform_micros(anchor_micros));
            controller.focus_waveform_context();
        }
        NativeUiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        } => controller.set_waveform_selection_range_micros_with_drag_policy(
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        ),
        NativeUiAction::SetWaveformSelectionRangeSmartScale {
            start_micros,
            end_micros,
        } => controller.set_waveform_selection_range_micros_smart_scale(start_micros, end_micros),
        NativeUiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        } => controller.set_waveform_edit_selection_range_micros_with_edge_policy(
            start_micros,
            end_micros,
            preserve_view_edge,
        ),
        NativeUiAction::SetWaveformEditFadeInEnd { position_micros } => {
            controller.set_waveform_edit_fade_in_end_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
            controller.set_waveform_edit_fade_in_mute_start_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeInCurve { curve_milli } => {
            controller.set_waveform_edit_fade_in_curve_milli(curve_milli)
        }
        NativeUiAction::SetWaveformEditFadeOutStart { position_micros } => {
            controller.set_waveform_edit_fade_out_start_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
            controller.set_waveform_edit_fade_out_mute_end_micros(position_micros)
        }
        NativeUiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
            controller.set_waveform_edit_fade_out_curve_milli(curve_milli)
        }
        NativeUiAction::FinishWaveformEditFadeDrag => controller.finish_waveform_edit_fade_drag(),
        NativeUiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        } => start_waveform_selection_drag(controller, pointer_x, pointer_y),
        NativeUiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            over_browser_list,
            shift_down,
            alt_down,
        } => controller.update_active_drag(
            native_drag_point(pointer_x, pointer_y),
            DragSource::Browser,
            if let Some(pane) = hovered_folder_pane.or(over_folder_panel) {
                DragTarget::FolderPanel {
                    pane: folder_pane_id_from_native(pane),
                    folder: hovered_folder_row
                        .and_then(|row| folder_row_path(controller, pane, row)),
                }
            } else if over_browser_list {
                DragTarget::BrowserList
            } else {
                DragTarget::None
            },
            shift_down,
            alt_down,
        ),
        NativeUiAction::FinishWaveformSelectionDrag => controller.finish_active_drag(),
        NativeUiAction::FinishWaveformSelectionRangeDrag => controller.finish_selection_drag(),
        NativeUiAction::FinishWaveformSelectionSmartScaleDrag => controller.finish_selection_drag(),
        NativeUiAction::FinishWaveformEditSelectionDrag => controller.finish_edit_selection_drag(),
        action => return Err(action),
    }
    Ok(())
}

fn start_waveform_selection_drag(controller: &mut AppController, pointer_x: u16, pointer_y: u16) {
    let Some(bounds) = controller.ui.waveform.selection else {
        return;
    };
    controller.start_selection_drag_payload(bounds, native_drag_point(pointer_x, pointer_y), true);
    controller.ui.drag.origin_source = Some(DragSource::Waveform);
}

fn normalize_waveform_micros(position_micros: u32) -> f32 {
    position_micros.min(1_000_000) as f32 / 1_000_000.0
}

/// Convert native action pointer coordinates into controller UI points.
fn native_drag_point(pointer_x: u16, pointer_y: u16) -> UiPoint {
    UiPoint::new(f32::from(pointer_x), f32::from(pointer_y))
}

fn folder_pane_id_from_native(pane: FolderPaneIdModel) -> FolderPaneId {
    match pane {
        FolderPaneIdModel::Upper => FolderPaneId::Upper,
        FolderPaneIdModel::Lower => FolderPaneId::Lower,
    }
}

fn folder_row_path(
    controller: &AppController,
    pane: FolderPaneIdModel,
    row: usize,
) -> Option<std::path::PathBuf> {
    let pane = folder_pane_id_from_native(pane);
    if controller.active_folder_pane() == pane {
        controller
            .ui
            .sources
            .folders
            .rows
            .get(row)
            .map(|folder| folder.path.clone())
    } else {
        controller
            .ui
            .sources
            .folder_pane(pane)
            .browser
            .rows
            .get(row)
            .map(|folder| folder.path.clone())
    }
}
