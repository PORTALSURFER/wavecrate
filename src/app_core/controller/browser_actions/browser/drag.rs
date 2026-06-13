use super::super::super::AppController;
use crate::app_core::actions::{NativeFolderPaneIdModel as FolderPaneIdModel, NativeUiAction};
use crate::app_core::app_api::state::{
    DragSource, DragTarget, FolderBrowserUiState, FolderPaneId, UiPoint,
};

pub(super) fn apply_drag_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            },
        ) => controller
            .start_browser_sample_drag_action(visible_row, native_drag_point(pointer_x, pointer_y)),
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            },
        ) => {
            let target = folder_drag_target(
                controller,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
            );
            controller.update_active_drag(
                native_drag_point(pointer_x, pointer_y),
                DragSource::Browser,
                target,
                shift_down,
                alt_down,
            );
        }
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FinishBrowserSampleDrag,
        ) => controller.finish_active_drag(),
        action => return Err(action),
    }
    Ok(())
}

fn folder_drag_target(
    controller: &AppController,
    hovered_folder_pane: Option<FolderPaneIdModel>,
    hovered_folder_row: Option<usize>,
    over_folder_panel: Option<FolderPaneIdModel>,
) -> DragTarget {
    if let Some(folder) = hovered_folder_pane
        .zip(hovered_folder_row)
        .and_then(|(pane, row)| folder_row_path(controller, pane, row))
    {
        return DragTarget::FolderPanel {
            pane: hovered_folder_pane
                .map(folder_pane_id_from_native)
                .unwrap_or_else(|| controller.active_folder_pane()),
            folder: Some(folder),
        };
    }

    if let Some(pane) = over_folder_panel.map(folder_pane_id_from_native) {
        DragTarget::FolderPanel { pane, folder: None }
    } else {
        DragTarget::None
    }
}

fn folder_row_path(
    controller: &AppController,
    pane: FolderPaneIdModel,
    folder_row: usize,
) -> Option<std::path::PathBuf> {
    folder_browser_for_pane(controller, pane)
        .rows
        .get(folder_row)
        .map(|row| row.path.clone())
}

fn native_drag_point(pointer_x: u16, pointer_y: u16) -> UiPoint {
    UiPoint::new(f32::from(pointer_x), f32::from(pointer_y))
}

fn folder_pane_id_from_native(pane: FolderPaneIdModel) -> FolderPaneId {
    match pane {
        FolderPaneIdModel::Upper => FolderPaneId::Upper,
        FolderPaneIdModel::Lower => FolderPaneId::Lower,
    }
}

fn folder_browser_for_pane(
    controller: &AppController,
    pane: FolderPaneIdModel,
) -> &FolderBrowserUiState {
    let pane = folder_pane_id_from_native(pane);
    if controller.active_folder_pane() == pane {
        &controller.ui.sources.folders
    } else {
        &controller.ui.sources.folder_pane(pane).browser
    }
}
