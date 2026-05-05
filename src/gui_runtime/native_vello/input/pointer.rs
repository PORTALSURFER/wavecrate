use super::*;
use crate::gui::{
    list::{EditableRowKind, EditableTreeRow},
    panel::{SplitPaneSlot, SplitPaneTreePanel},
};

pub(super) fn action_from_pointer_with_motion(
    layout: &ShellLayout,
    model: &AppModel,
    motion_model: Option<&NativeMotionModel>,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    route_modal_and_chrome_actions(layout, model, motion_model, shell_state, point, modifiers)
        .or_else(|| route_browser_or_folder_row(layout, model, shell_state, point, modifiers))
        .or_else(|| route_shell_background(layout, model, shell_state, point, modifiers))
}

fn route_modal_and_chrome_actions(
    layout: &ShellLayout,
    model: &AppModel,
    motion_model: Option<&NativeMotionModel>,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.prompt_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.progress_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.options_panel_action_at_point(layout, model, point) {
        return Some(action);
    }
    if model.options_panel.visible {
        if shell_state.options_panel_contains_point_live(layout, model, point) {
            return None;
        }
        return Some(UiAction::CloseOptionsPanel);
    }
    if let Some(action) = shell_state.status_options_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_update_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_volume_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_content_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) =
        shell_state.browser_action_at_point(layout, model, point, modifiers.alt_key())
    {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.folder_header_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = motion_model.and_then(|motion_model| {
        shell_state.waveform_toolbar_action_at_point_with_motion_and_modifiers(
            layout,
            motion_model,
            point,
            modifiers.shift_key(),
        )
    }) {
        return Some(action);
    }
    shell_state.waveform_toolbar_action_at_point_with_modifiers(
        layout,
        model,
        point,
        modifiers.shift_key(),
    )
}

fn route_browser_or_folder_row(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.browser_row_similarity_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        let shift = modifiers.shift_key();
        let command = modifiers.control_key() || modifiers.super_key();
        return Some(if shift && command {
            UiAction::AddRangeBrowserSelection { visible_row }
        } else if shift {
            UiAction::ExtendBrowserSelectionToRow { visible_row }
        } else if command {
            UiAction::ToggleBrowserRowSelection { visible_row }
        } else {
            UiAction::FocusBrowserRow { visible_row }
        });
    }
    if let Some((pane, index)) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_disclosure_action(model, pane, index));
    }
    shell_state
        .folder_row_at_point(layout, model, point)
        .map(|(pane, index)| folder_row_body_action(model, pane, index))
}

fn route_shell_background(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => route_sidebar_background(layout, model, shell_state, point),
        ShellNodeKind::WaveformCard => {
            if layout.waveform_plot.contains(point) {
                Some(waveform_action_from_pointer(
                    layout, model, point, modifiers,
                ))
            } else {
                Some(UiAction::FocusWaveformPanel)
            }
        }
        ShellNodeKind::TopBar => Some(UiAction::ToggleTransport),
        ShellNodeKind::BrowserPanel | ShellNodeKind::BrowserTabs | ShellNodeKind::BrowserTable => {
            Some(UiAction::FocusBrowserPanel)
        }
        ShellNodeKind::StatusBar => Some(UiAction::FocusLoadedContentInList),
        _ => None,
    }
}

fn route_sidebar_background(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<UiAction> {
    if let Some((pane, index)) = shell_state.source_row_at_point(layout, model, point) {
        return Some(UiAction::FocusSourceRow {
            pane: Some(pane),
            index,
        });
    }
    if let Some((pane, index)) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_disclosure_action(model, pane, index));
    }
    if let Some((pane, index)) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(folder_row_body_action(model, pane, index));
    }
    shell_state.sidebar_focus_action_at_point(layout, model, point)
}

fn folder_row_disclosure_action(model: &AppModel, pane: SplitPaneSlot, index: usize) -> UiAction {
    let pane_model = model.sources.folder_pane(pane);
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return UiAction::FocusFolderRow {
            pane: Some(pane),
            index,
        };
    };
    if matches!(
        row.kind,
        EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
    ) {
        return UiAction::FocusFolderCreateInput;
    }
    let source_index = row.backing_index.unwrap_or(index);
    if folder_row_disclosure_toggles_expansion(pane_model, index) {
        UiAction::ToggleFolderRowExpanded {
            pane: Some(pane),
            index: source_index,
        }
    } else {
        UiAction::FocusFolderRow {
            pane: Some(pane),
            index: source_index,
        }
    }
}

fn folder_row_body_action(model: &AppModel, pane: SplitPaneSlot, index: usize) -> UiAction {
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return UiAction::FocusFolderRow {
            pane: Some(pane),
            index,
        };
    };
    if matches!(
        row.kind,
        EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
    ) {
        return UiAction::FocusFolderCreateInput;
    }
    let source_index = row.backing_index.unwrap_or(index);
    UiAction::FocusFolderRow {
        pane: Some(pane),
        index: source_index,
    }
}

fn folder_row_for_pointer_action(
    model: &AppModel,
    pane: SplitPaneSlot,
    index: usize,
) -> Option<&EditableTreeRow> {
    let pane_row = model.sources.folder_pane(pane).tree_rows.get(index);
    let flat_active_row = (pane == model.sources.active_folder_pane)
        .then(|| model.sources.tree_rows.get(index))
        .flatten();
    flat_active_row
        .filter(|row| {
            matches!(
                row.kind,
                EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
            )
        })
        .or(pane_row)
        .or(flat_active_row)
}

fn folder_row_disclosure_toggles_expansion(
    pane_model: &SplitPaneTreePanel<EditableTreeRow>,
    index: usize,
) -> bool {
    let Some(row) = pane_model.tree_rows.get(index) else {
        return false;
    };
    row.has_children
        && !row.is_root
        && !matches!(
            row.kind,
            EditableRowKind::CreateDraft | EditableRowKind::RenameDraft
        )
        && pane_model.tree_search_query.trim().is_empty()
}
