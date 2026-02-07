//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use crate::{
    egui_app::{
        controller::EguiController,
        state::{TriageFlagColumn, UiState},
        view_model,
    },
    selection::SelectionRange,
};
use radiant::app::{
    AppModel, BrowserActionsModel, BrowserPanelModel, BrowserRowModel, ColumnModel,
    ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel, FolderActionsModel,
    FolderRecoveryModel, FolderRowModel, NormalizedRangeModel, ProgressOverlayModel,
    SourceRowModel, SourcesPanelModel, StatusBarModel, WaveformPanelModel,
};
use std::collections::HashSet;

const MAX_RENDERED_BROWSER_ROWS: usize = 48;

pub(crate) fn project_app_model(controller: &mut EguiController) -> AppModel {
    let selected_column = selected_column_index(&controller.ui);
    let transport_running = controller.is_playing();
    let sources = project_sources_model(&controller.ui);
    let status_text = controller.ui.status.text.clone();
    let status = project_status_model(&controller.ui, selected_column);
    let browser_actions = project_browser_actions_model(&controller.ui);
    let progress_overlay = project_progress_overlay_model(&controller.ui);
    let confirm_prompt = project_confirm_prompt_model(&controller.ui);
    let drag_overlay = project_drag_overlay_model(&controller.ui);
    let column_counts = [
        controller.ui.browser.trash.len(),
        controller.ui.browser.neutral.len(),
        controller.ui.browser.keep.len(),
    ];
    let waveform = project_waveform_model(&controller.ui);
    let browser = project_browser_model(controller);
    AppModel {
        title: String::from("Sempal"),
        backend_label: String::from("backend: native_vello"),
        sources_label: format!("Sources ({})", sources.rows.len()),
        status_text,
        status,
        browser_actions,
        progress_overlay,
        confirm_prompt,
        drag_overlay,
        columns: [
            ColumnModel::new("Trash", column_counts[0]),
            ColumnModel::new("Samples", column_counts[1]),
            ColumnModel::new("Keep", column_counts[2]),
        ],
        selected_column,
        transport_running,
        sources,
        browser,
        waveform,
    }
}

fn project_browser_actions_model(ui: &UiState) -> BrowserActionsModel {
    let has_focus = ui.browser.selected_visible.is_some();
    let has_selection = has_focus || !ui.browser.selected_paths.is_empty();
    BrowserActionsModel {
        can_rename: has_focus,
        can_delete: has_selection,
        can_tag: has_selection,
    }
}

fn project_progress_overlay_model(ui: &UiState) -> ProgressOverlayModel {
    ProgressOverlayModel {
        visible: ui.progress.visible,
        modal: ui.progress.modal,
        title: ui.progress.title.clone(),
        detail: ui.progress.detail.clone(),
        completed: ui.progress.completed,
        total: ui.progress.total,
        cancelable: ui.progress.cancelable,
        cancel_requested: ui.progress.cancel_requested,
    }
}

fn project_confirm_prompt_model(ui: &UiState) -> ConfirmPromptModel {
    if let Some(crate::egui_app::state::SampleBrowserActionPrompt::Rename { target, .. }) =
        ui.browser.pending_action.as_ref()
    {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::BrowserRename),
            title: String::from("Rename sample"),
            message: String::from("Apply rename for focused sample?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
        };
    }
    if let Some(crate::egui_app::state::FolderActionPrompt::Rename { target, .. }) =
        ui.sources.folders.pending_action.as_ref()
    {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::FolderRename),
            title: String::from("Rename folder"),
            message: String::from("Apply folder rename?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
        };
    }
    if let Some(prompt) = ui.waveform.pending_destructive.as_ref() {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::DestructiveEdit),
            title: prompt.title.clone(),
            message: prompt.message.clone(),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: None,
        };
    }
    ConfirmPromptModel::default()
}

fn project_drag_overlay_model(ui: &UiState) -> DragOverlayModel {
    let active = ui.drag.payload.is_some();
    if !active {
        return DragOverlayModel::default();
    }
    let target_label = match &ui.drag.active_target {
        crate::egui_app::state::DragTarget::None => String::from("No target"),
        crate::egui_app::state::DragTarget::BrowserTriage(column) => match column {
            crate::egui_app::state::TriageFlagColumn::Trash => String::from("Trash column"),
            crate::egui_app::state::TriageFlagColumn::Neutral => String::from("Neutral column"),
            crate::egui_app::state::TriageFlagColumn::Keep => String::from("Keep column"),
        },
        crate::egui_app::state::DragTarget::SourcesRow(_) => String::from("Sources list"),
        crate::egui_app::state::DragTarget::FolderPanel { folder } => folder
            .as_ref()
            .map(|path| format!("Folder: {}", path.display()))
            .unwrap_or_else(|| String::from("Folder panel")),
        crate::egui_app::state::DragTarget::DropTarget { path } => {
            format!("Drop target: {}", path.display())
        }
        crate::egui_app::state::DragTarget::DropTargetsPanel => String::from("Drop targets"),
        crate::egui_app::state::DragTarget::External => String::from("External target"),
    };
    DragOverlayModel {
        active,
        label: ui.drag.label.clone(),
        target_label,
        valid_target: !matches!(
            ui.drag.active_target,
            crate::egui_app::state::DragTarget::None
        ),
    }
}

fn project_status_model(ui: &UiState, selected_column: usize) -> StatusBarModel {
    let left = ui.status.text.clone();
    let center = format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
        ui.browser.visible.len(),
        ui.browser.selected_paths.len(),
        ui.browser
            .selection_anchor_visible
            .map(|row| row.to_string())
            .unwrap_or_else(|| String::from("—")),
        if ui.browser.search_query.is_empty() {
            "—"
        } else {
            ui.browser.search_query.as_str()
        },
        if ui.browser.search_busy {
            " | filtering…"
        } else {
            ""
        }
    );
    let right = format!("col: {}/3", selected_column + 1);
    StatusBarModel {
        left,
        center,
        right,
    }
}

pub(crate) fn selected_column_index(ui: &UiState) -> usize {
    ui.browser
        .selected
        .map(|selected| match selected.column {
            TriageFlagColumn::Trash => 0,
            TriageFlagColumn::Neutral => 1,
            TriageFlagColumn::Keep => 2,
        })
        .unwrap_or(1)
}

pub(crate) fn browser_focus_target(ui: &UiState, delta: i8) -> Option<usize> {
    let visible_count = ui.browser.visible.len();
    if visible_count == 0 {
        return None;
    }
    let base = ui
        .browser
        .selected_visible
        .unwrap_or(0)
        .min(visible_count - 1);
    Some((base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize)
}

pub(crate) fn normalized_from_milli(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

pub(crate) fn selection_range_from_milli(start_milli: u16, end_milli: u16) -> SelectionRange {
    SelectionRange::new(
        normalized_from_milli(start_milli),
        normalized_from_milli(end_milli),
    )
}

fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
    let focused_folder = ui
        .sources
        .folders
        .focused
        .and_then(|index| ui.sources.folders.rows.get(index).cloned());
    let can_manage_folder = focused_folder.as_ref().is_some_and(|row| !row.is_root);
    SourcesPanelModel {
        header: format!("Sources ({})", ui.sources.rows.len()),
        search_query: ui.sources.folders.search_query.clone(),
        folder_search_query: ui.sources.folders.search_query.clone(),
        selected_row: ui.sources.selected,
        focused_folder_row: ui.sources.folders.focused,
        rows: ui
            .sources
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                SourceRowModel::new(
                    row.name.clone(),
                    row.path.clone(),
                    ui.sources
                        .selected
                        .is_some_and(|selected| selected == row_index),
                    row.missing,
                )
            })
            .collect(),
        folder_rows: ui
            .sources
            .folders
            .rows
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                FolderRowModel::new(
                    row.name.clone(),
                    row.path.display().to_string(),
                    row.depth,
                    row.selected,
                    ui.sources
                        .folders
                        .focused
                        .is_some_and(|focused| focused == row_index),
                    row.is_root,
                    row.has_children,
                    row.expanded,
                )
            })
            .collect(),
        folder_actions: FolderActionsModel {
            can_create_folder: ui.sources.selected.is_some(),
            can_create_folder_at_root: ui.sources.selected.is_some(),
            can_rename_folder: can_manage_folder,
            can_delete_folder: can_manage_folder,
            can_clear_recovery_log: !ui.sources.folders.delete_recovery.entries.is_empty(),
        },
        folder_recovery: FolderRecoveryModel {
            in_progress: ui.sources.folders.delete_recovery.in_progress,
            entry_count: ui.sources.folders.delete_recovery.entries.len(),
        },
    }
}

fn project_browser_model(controller: &mut EguiController) -> BrowserPanelModel {
    let visible = controller.ui.browser.visible.clone();
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let busy = controller.ui.browser.search_busy;
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let selected_paths: HashSet<_> = controller
        .ui
        .browser
        .selected_paths
        .iter()
        .cloned()
        .collect();

    let mut rows = Vec::new();
    let visible_count = visible.len();
    let rendered = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    for visible_row in 0..rendered {
        let Some(absolute_index) = visible.get(visible_row) else {
            continue;
        };
        if let Some(entry) = controller.wav_entry(absolute_index) {
            let selected = selected_paths.contains(&entry.relative_path);
            rows.push(BrowserRowModel::new(
                visible_row,
                view_model::sample_display_label(&entry.relative_path),
                browser_column_index(entry.tag),
                selected,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        } else {
            rows.push(BrowserRowModel::new(
                visible_row,
                format!("row {}", visible_row + 1),
                1,
                false,
                selected_visible_row.is_some_and(|focused| focused == visible_row),
            ));
        }
    }

    BrowserPanelModel {
        visible_count,
        selected_visible_row,
        selected_path_count,
        search_query,
        busy,
        focused_sample_label,
        anchor_visible_row,
        rows,
    }
}

fn project_waveform_model(ui: &UiState) -> WaveformPanelModel {
    WaveformPanelModel {
        loaded_label: ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        cursor_milli: ui.waveform.cursor.map(normalized_to_milli),
        playhead_milli: ui
            .waveform
            .playhead
            .visible
            .then_some(normalized_to_milli(ui.waveform.playhead.position)),
        selection_milli: ui.waveform.selection.map(|selection| {
            NormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        view_start_milli: normalized64_to_milli(ui.waveform.view.start),
        view_end_milli: normalized64_to_milli(ui.waveform.view.end),
        loop_enabled: ui.waveform.loop_enabled,
    }
}

fn browser_column_index(tag: crate::sample_sources::Rating) -> usize {
    if tag.is_trash() {
        0
    } else if tag.is_keep() {
        2
    } else {
        1
    }
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_column_defaults_to_middle_column_without_selection() {
        let ui = UiState::default();
        assert_eq!(selected_column_index(&ui), 1);
    }

    #[test]
    fn normalized_from_milli_clamps_bounds() {
        assert_eq!(normalized_from_milli(0), 0.0);
        assert_eq!(normalized_from_milli(455), 0.455);
        assert_eq!(normalized_from_milli(2000), 1.0);
    }

    #[test]
    fn browser_focus_target_clamps_to_visible_window() {
        let mut ui = UiState::default();
        ui.browser.visible = crate::egui_app::state::VisibleRows::List(vec![0, 1, 2, 3]);
        ui.browser.selected_visible = Some(1);

        assert_eq!(browser_focus_target(&ui, -8), Some(0));
        assert_eq!(browser_focus_target(&ui, 1), Some(2));
        assert_eq!(browser_focus_target(&ui, 99), Some(3));
    }

    #[test]
    fn browser_column_index_maps_rating_buckets() {
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::TRASH_1),
            0
        );
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::NEUTRAL),
            1
        );
        assert_eq!(
            browser_column_index(crate::sample_sources::Rating::KEEP_1),
            2
        );
    }

    #[test]
    fn selection_range_from_milli_clamps_and_orders_bounds() {
        let range = selection_range_from_milli(750, 250);
        assert_eq!(range.start(), 0.25);
        assert_eq!(range.end(), 0.75);

        let range = selection_range_from_milli(2000, 0);
        assert_eq!(range.start(), 0.0);
        assert_eq!(range.end(), 1.0);
    }

    #[test]
    fn browser_actions_require_focus_or_selection() {
        let mut ui = UiState::default();
        let projected = project_browser_actions_model(&ui);
        assert!(!projected.can_rename);
        assert!(!projected.can_delete);
        assert!(!projected.can_tag);

        ui.browser.selected_visible = Some(0);
        let projected = project_browser_actions_model(&ui);
        assert!(projected.can_rename);
        assert!(projected.can_delete);
        assert!(projected.can_tag);
    }

    #[test]
    fn confirm_prompt_prefers_browser_rename_when_multiple_prompts_exist() {
        let mut ui = UiState::default();
        ui.browser.pending_action =
            Some(crate::egui_app::state::SampleBrowserActionPrompt::Rename {
                target: std::path::PathBuf::from("kick.wav"),
                name: String::from("kick"),
            });
        ui.waveform.pending_destructive = Some(crate::egui_app::state::DestructiveEditPrompt {
            edit: crate::egui_app::state::DestructiveSelectionEdit::TrimSelection,
            title: String::from("Trim selection"),
            message: String::from("Apply trim?"),
        });
        let projected = project_confirm_prompt_model(&ui);
        assert!(projected.visible);
        assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserRename));
    }

    #[test]
    fn progress_overlay_projection_preserves_cancel_state() {
        let mut ui = UiState::default();
        ui.progress.visible = true;
        ui.progress.modal = true;
        ui.progress.title = String::from("Scanning");
        ui.progress.completed = 3;
        ui.progress.total = 9;
        ui.progress.cancelable = true;
        ui.progress.cancel_requested = true;
        let projected = project_progress_overlay_model(&ui);
        assert!(projected.visible);
        assert!(projected.modal);
        assert!(projected.cancelable);
        assert!(projected.cancel_requested);
        assert_eq!(projected.completed, 3);
        assert_eq!(projected.total, 9);
    }

    #[test]
    fn folder_actions_require_non_root_focus_for_destructive_actions() {
        let mut ui = UiState::default();
        ui.sources.selected = Some(0);
        ui.sources
            .folders
            .rows
            .push(crate::egui_app::state::FolderRowView {
                path: std::path::PathBuf::new(),
                name: String::from("Root"),
                depth: 0,
                has_children: true,
                expanded: true,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: true,
                root_filter_mode: None,
            });
        ui.sources.folders.focused = Some(0);
        let projected = project_sources_model(&ui);
        assert!(projected.folder_actions.can_create_folder);
        assert!(projected.folder_actions.can_create_folder_at_root);
        assert!(!projected.folder_actions.can_rename_folder);
        assert!(!projected.folder_actions.can_delete_folder);

        ui.sources
            .folders
            .rows
            .push(crate::egui_app::state::FolderRowView {
                path: std::path::PathBuf::from("drums"),
                name: String::from("drums"),
                depth: 1,
                has_children: false,
                expanded: false,
                selected: true,
                negated: false,
                hotkey: None,
                is_root: false,
                root_filter_mode: None,
            });
        ui.sources.folders.focused = Some(1);
        let projected = project_sources_model(&ui);
        assert!(projected.folder_actions.can_rename_folder);
        assert!(projected.folder_actions.can_delete_folder);
    }
}
