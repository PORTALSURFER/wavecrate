//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use super::controller::AppController;
use crate::app_core::actions::{
    NativeAppModel as AppModel, NativeBrowserActionsModel as BrowserActionsModel,
    NativeBrowserChromeModel as BrowserChromeModel, NativeBrowserPanelModel as BrowserPanelModel,
    NativeBrowserRowModel as BrowserRowModel, NativeColumnModel as ColumnModel,
    NativeConfirmPromptKind as ConfirmPromptKind, NativeConfirmPromptModel as ConfirmPromptModel,
    NativeDragOverlayModel as DragOverlayModel, NativeFolderActionsModel as FolderActionsModel,
    NativeFolderRecoveryModel as FolderRecoveryModel, NativeFolderRowModel as FolderRowModel,
    NativeMapPanelModel as MapPanelModel, NativeMapPointModel as MapPointModel,
    NativeMapRenderModeModel as MapRenderModeModel, NativeMotionModel as MotionModel,
    NativeNormalizedRangeModel as NormalizedRangeModel,
    NativeProgressOverlayModel as ProgressOverlayModel, NativeSourceRowModel as SourceRowModel,
    NativeSourcesPanelModel as SourcesPanelModel, NativeStatusBarModel as StatusBarModel,
    NativeUpdatePanelModel as UpdatePanelModel, NativeUpdateStatusModel as UpdateStatusModel,
    NativeWaveformChromeModel as WaveformChromeModel,
    NativeWaveformPanelModel as WaveformPanelModel,
};
use crate::gui::types::ImageRgba;
use crate::app_core::state::{
    DestructiveEditPrompt, DragTarget, FolderActionPrompt, MapQueryBounds, MapRenderMode,
    SampleBrowserActionPrompt, SampleBrowserSort, SampleBrowserTab, TriageFlagColumn, UiState,
    UpdateStatus,
};
#[cfg(test)]
use crate::app_core::state::{
    DestructiveSelectionEdit, FolderDeleteRecoveryAction, FolderDeleteRecoveryEntry,
    FolderDeleteRecoveryStatus, FolderRowView, InlineFolderCreation,
};
use crate::app_core::ui::{MAX_RENDERED_BROWSER_ROWS, MAX_RENDERED_MAP_POINTS};
use crate::{analysis::similarity::SIMILARITY_MODEL_ID, app_core::view_model};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::info;

static PROJECT_APP_MODEL_CALLS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn project_app_model(controller: &mut AppController) -> AppModel {
    let call = PROJECT_APP_MODEL_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if call <= 12 {
        let status_len = controller.ui.status.text.len();
        info!(
            call,
            selected_column = selected_column_index(&controller.ui),
            status_len,
            visible_browser_rows = controller.ui.browser.visible.len(),
            "native shell: project_app_model start"
        );
    }
    let selected_column = selected_column_index(&controller.ui);
    let transport_running = controller.is_playing();
    let sources = project_sources_model(&controller.ui);
    let status_text = controller.ui.status.text.clone();
    let status = project_status_model(controller, selected_column);
    let browser_actions = project_browser_actions_model(&controller.ui);
    let map = project_map_model(controller);
    let update = project_update_model(&controller.ui);
    let progress_overlay = project_progress_overlay_model(&controller.ui);
    let confirm_prompt = project_confirm_prompt_model(&controller.ui);
    let drag_overlay = project_drag_overlay_model(&controller.ui);
    let column_counts = [
        controller.ui.browser.trash.len(),
        controller.ui.browser.neutral.len(),
        controller.ui.browser.keep.len(),
    ];
    let waveform = project_waveform_model(controller);
    let browser = project_browser_model(controller);
    let browser_chrome = project_browser_chrome_model(&controller.ui, browser.visible_count);
    let waveform_chrome = project_waveform_chrome_model(&controller.ui);
    let app_model = AppModel {
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
        browser_chrome,
        map,
        waveform,
        waveform_chrome,
        update,
    };
    if call <= 12 {
        info!(
            call,
            browser_visible = app_model.browser.visible_count,
            status_center_len = app_model.status.center.len(),
            transport_running = app_model.transport_running,
            "native shell: project_app_model complete"
        );
    }
    app_model
}

pub(crate) fn project_motion_model(controller: &mut AppController) -> MotionModel {
    let selected_column = selected_column_index(&controller.ui);
    let status = project_status_model(controller, selected_column);
    MotionModel {
        transport_running: controller.is_playing(),
        map_active: matches!(
            SampleBrowserTab::from(controller.ui.browser.active_tab),
            SampleBrowserTab::Map
        ),
        waveform_selection_milli: controller.ui.waveform.selection.map(|selection| {
            crate::app_core::actions::NativeNormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        waveform_cursor_milli: controller.ui.waveform.cursor.map(normalized_to_milli),
        waveform_playhead_milli: controller.ui.waveform.playhead.visible.then_some(
            normalized_to_milli(controller.ui.waveform.playhead.position),
        ),
        waveform_view_start_milli: normalized64_to_milli(controller.ui.waveform.view.start),
        waveform_view_end_milli: normalized64_to_milli(controller.ui.waveform.view.end),
        waveform_tempo_label: controller
            .ui
            .waveform
            .bpm_value
            .map(|bpm| format!("{bpm:.1} BPM")),
        waveform_zoom_label: Some(format!(
            "{:.0}%",
            (100.0
                / (controller.ui.waveform.view.end - controller.ui.waveform.view.start)
                    .clamp(0.000_1, 1.0) as f32)
                .round()
                .clamp(100.0, 9999.0)
        )),
        waveform_image_signature: controller.ui.waveform.waveform_image_signature,
        waveform_loaded_label: controller
            .ui
            .loaded_wav
            .as_deref()
            .map(view_model::sample_display_label),
        waveform_transport_hint: if controller.ui.waveform.loop_enabled {
            String::from("Loop enabled")
        } else {
            String::from("Loop disabled")
        },
        status_right: status.right,
    }
}

fn project_update_model(ui: &UiState) -> UpdatePanelModel {
    let update_status = UpdateStatus::from(ui.update.status.clone());
    let status = match update_status {
        UpdateStatus::Idle => UpdateStatusModel::Idle,
        UpdateStatus::Checking => UpdateStatusModel::Checking,
        UpdateStatus::UpdateAvailable => UpdateStatusModel::Available,
        UpdateStatus::Error => UpdateStatusModel::Error,
    };
    let status_label = match update_status {
        UpdateStatus::Idle => String::from("Updates: idle"),
        UpdateStatus::Checking => String::from("Checking updates..."),
        UpdateStatus::UpdateAvailable => ui
            .update
            .available_tag
            .as_deref()
            .map(|tag| format!("Update available: {tag}"))
            .unwrap_or_else(|| String::from("Update available")),
        UpdateStatus::Error => ui
            .update
            .last_error
            .as_deref()
            .map(|err| format!("Update check failed: {err}"))
            .unwrap_or_else(|| String::from("Update check failed")),
    };
    let action_hint_label = match update_status {
        UpdateStatus::Idle => String::from("Action: check"),
        UpdateStatus::Checking => String::from("Action: waiting"),
        UpdateStatus::UpdateAvailable => {
            if ui.update.available_url.is_some() {
                String::from("Actions: open | install | dismiss")
            } else {
                String::from("Action: dismiss")
            }
        }
        UpdateStatus::Error => String::from("Action: retry"),
    };
    let release_notes_label = match update_status {
        UpdateStatus::UpdateAvailable => {
            let tag = ui.update.available_tag.as_deref().unwrap_or("latest");
            if let Some(published_at) = ui.update.available_published_at.as_deref() {
                format!("Release: {tag} ({published_at})")
            } else {
                format!("Release: {tag}")
            }
        }
        _ => String::new(),
    };
    UpdatePanelModel {
        status,
        status_label,
        action_hint_label,
        release_notes_label,
        available_tag: ui.update.available_tag.clone(),
        available_url: ui.update.available_url.clone(),
        last_error: ui.update.last_error.clone(),
    }
}

fn project_map_model(controller: &mut AppController) -> MapPanelModel {
    let active = matches!(
        SampleBrowserTab::from(controller.ui.browser.active_tab),
        SampleBrowserTab::Map
    );
    let render_mode = match MapRenderMode::from(controller.ui.map.last_render_mode) {
        MapRenderMode::Heatmap => MapRenderModeModel::Heatmap,
        MapRenderMode::Points => MapRenderModeModel::Points,
    };
    let render_mode_label = match render_mode {
        MapRenderModeModel::Heatmap => "heatmap",
        MapRenderModeModel::Points => "points",
    };
    if !active {
        return MapPanelModel {
            active: false,
            summary: String::from("Map hidden"),
            legend_label: format!("Render: {render_mode_label}"),
            selection_label: String::from("Selection: —"),
            hover_label: String::from("Hover: —"),
            cluster_label: String::from("Clusters: —"),
            viewport_label: String::from("zoom 1.00x | pan (0, 0)"),
            error: None,
            render_mode,
            points: Vec::new(),
        };
    }

    let source_id = controller.current_source().map(|source| source.id);
    let umap_version = controller.ui.map.umap_version.clone();
    let bounds =
        match controller.umap_bounds(SIMILARITY_MODEL_ID, &umap_version, source_id.as_ref()) {
            Ok(bounds) => bounds,
            Err(err) => {
                return MapPanelModel {
                    active: true,
                    summary: String::from("Map unavailable"),
                    legend_label: format!("Render: {render_mode_label}"),
                    selection_label: String::from("Selection: unavailable"),
                    hover_label: String::from("Hover: unavailable"),
                    cluster_label: String::from("Clusters: unavailable"),
                    viewport_label: format!(
                        "zoom {:.2}x | pan ({:.0}, {:.0})",
                        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
                    ),
                    error: Some(err),
                    render_mode,
                    points: Vec::new(),
                };
            }
        };
    let Some(bounds) = bounds else {
        return MapPanelModel {
            active: true,
            summary: String::from("No map data (run similarity prep)"),
            legend_label: format!("Render: {render_mode_label}"),
            selection_label: String::from("Selection: —"),
            hover_label: String::from("Hover: —"),
            cluster_label: String::from("Clusters: —"),
            viewport_label: format!(
                "zoom {:.2}x | pan ({:.0}, {:.0})",
                controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
            ),
            error: None,
            render_mode,
            points: Vec::new(),
        };
    };

    let points = match controller.umap_points_in_bounds(
        SIMILARITY_MODEL_ID,
        &umap_version,
        "umap",
        &umap_version,
        source_id.as_ref(),
        MapQueryBounds {
            min_x: bounds.min_x,
            max_x: bounds.max_x,
            min_y: bounds.min_y,
            max_y: bounds.max_y,
        }
        .into(),
        MAX_RENDERED_MAP_POINTS,
    ) {
        Ok(points) => points,
        Err(err) => {
            return MapPanelModel {
                active: true,
                summary: String::from("Map query failed"),
                legend_label: format!("Render: {render_mode_label}"),
                selection_label: String::from("Selection: unavailable"),
                hover_label: String::from("Hover: unavailable"),
                cluster_label: String::from("Clusters: unavailable"),
                viewport_label: format!(
                    "zoom {:.2}x | pan ({:.0}, {:.0})",
                    controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
                ),
                error: Some(err),
                render_mode,
                points: Vec::new(),
            };
        }
    };

    let focused_sample_id = controller.selected_sample_id();
    let selected_sample_id = controller.ui.map.selected_sample_id.clone();
    let denom_x = (bounds.max_x - bounds.min_x).max(1e-6);
    let denom_y = (bounds.max_y - bounds.min_y).max(1e-6);
    let cluster_count = points
        .iter()
        .filter_map(|point| point.cluster_id)
        .collect::<HashSet<_>>()
        .len();
    let points = points
        .into_iter()
        .map(|point| {
            let x = ((point.x - bounds.min_x) / denom_x).clamp(0.0, 1.0);
            let y = ((point.y - bounds.min_y) / denom_y).clamp(0.0, 1.0);
            MapPointModel {
                selected: selected_sample_id
                    .as_deref()
                    .is_some_and(|selected| selected == point.sample_id),
                focused: focused_sample_id
                    .as_deref()
                    .is_some_and(|focused| focused == point.sample_id),
                sample_id: point.sample_id,
                x_milli: normalized_to_milli(x),
                y_milli: normalized_to_milli(y),
                cluster_id: point.cluster_id,
            }
        })
        .collect::<Vec<_>>();
    let selection_label = controller
        .ui
        .map
        .selected_sample_id
        .as_deref()
        .map(short_sample_label)
        .map(|label| format!("Selection: {label}"))
        .or_else(|| {
            focused_sample_id
                .as_deref()
                .map(short_sample_label)
                .map(|label| format!("Focus: {label}"))
        })
        .unwrap_or_else(|| String::from("Selection: —"));
    let hover_label = controller
        .ui
        .map
        .hovered_sample_id
        .as_deref()
        .or(controller.ui.map.paint_hover_active_id.as_deref())
        .map(short_sample_label)
        .map(|label| format!("Hover: {label}"))
        .unwrap_or_else(|| String::from("Hover: —"));
    let cluster_label = if cluster_count == 0 {
        String::from("Clusters: —")
    } else {
        format!("Clusters: {cluster_count}")
    };
    let viewport_label = format!(
        "zoom {:.2}x | pan ({:.0}, {:.0})",
        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
    );
    let summary = format!("{} points", points.len());
    MapPanelModel {
        active: true,
        summary,
        legend_label: format!("Render: {render_mode_label}"),
        selection_label,
        hover_label,
        cluster_label,
        viewport_label,
        error: None,
        render_mode,
        points,
    }
}

fn short_sample_label(sample_id: &str) -> String {
    let candidate = Path::new(sample_id)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(sample_id);
    if candidate.chars().count() > 32 {
        let mut truncated = candidate.chars().take(29).collect::<String>();
        truncated.push_str("...");
        truncated
    } else {
        candidate.to_string()
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
    if let Some(SampleBrowserActionPrompt::Rename { target, name }) = ui
        .browser
        .pending_action
        .clone()
        .map(SampleBrowserActionPrompt::from)
    {
        let input_value = Some(name);
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::BrowserRename),
            title: String::from("Rename sample"),
            message: String::from("Apply rename for focused sample?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
            input_value,
            input_placeholder: Some(String::from("Sample name")),
            input_error: None,
        };
    }
    if let Some(FolderActionPrompt::Rename { target, name }) = ui
        .sources
        .folders
        .pending_action
        .clone()
        .map(FolderActionPrompt::from)
    {
        let input_value = Some(name);
        let input_error = input_value
            .as_deref()
            .and_then(|name| folder_rename_validation_error(ui, &target, name));
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::FolderRename),
            title: String::from("Rename folder"),
            message: String::from("Apply folder rename?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
            input_value,
            input_placeholder: Some(String::from("Folder name")),
            input_error,
        };
    }
    if let Some(new_folder) = ui.sources.folders.new_folder.as_ref() {
        let target_label = if new_folder.parent.as_os_str().is_empty() {
            String::from("source root")
        } else {
            new_folder.parent.display().to_string()
        };
        let input_error = folder_create_validation_error(ui, &new_folder.parent, &new_folder.name);
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::FolderCreate),
            title: String::from("Create folder"),
            message: String::from("Create a new folder at the selected location."),
            confirm_label: String::from("Create"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target_label),
            input_value: Some(new_folder.name.clone()),
            input_placeholder: Some(String::from("New folder name")),
            input_error,
        };
    }
    if let Some(prompt) = ui
        .waveform
        .pending_destructive
        .clone()
        .map(DestructiveEditPrompt::from)
    {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::DestructiveEdit),
            title: prompt.title,
            message: prompt.message,
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        };
    }
    ConfirmPromptModel::default()
}

fn project_drag_overlay_model(ui: &UiState) -> DragOverlayModel {
    let active = ui.drag.payload.is_some();
    if !active {
        return DragOverlayModel::default();
    }
    let active_target = DragTarget::from(ui.drag.active_target.clone());
    let target_label = match &active_target {
        DragTarget::None => String::from("No target"),
        DragTarget::BrowserTriage(column) => match TriageFlagColumn::from(*column) {
            TriageFlagColumn::Trash => String::from("Trash column"),
            TriageFlagColumn::Neutral => String::from("Neutral column"),
            TriageFlagColumn::Keep => String::from("Keep column"),
        },
        DragTarget::SourcesRow(_) => String::from("Sources list"),
        DragTarget::FolderPanel { folder } => folder
            .as_ref()
            .map(|path| format!("Folder: {}", path.display()))
            .unwrap_or_else(|| String::from("Folder panel")),
        DragTarget::DropTarget { path } => {
            format!("Drop target: {}", path.display())
        }
        DragTarget::DropTargetsPanel => String::from("Drop targets"),
        DragTarget::External => String::from("External target"),
    };
    DragOverlayModel {
        active,
        label: ui.drag.label.clone(),
        target_label,
        valid_target: !matches!(active_target, DragTarget::None),
    }
}

fn project_status_model(controller: &AppController, selected_column: usize) -> StatusBarModel {
    let left = controller.ui.status.text.clone();
    let center = format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
        controller.ui.browser.visible.len(),
        controller.ui.browser.selected_paths.len(),
        controller
            .ui
            .browser
            .selection_anchor_visible
            .map(|row: usize| row.to_string())
            .unwrap_or_else(|| String::from("—")),
        if controller.ui.browser.search_query.is_empty() {
            "—"
        } else {
            controller.ui.browser.search_query.as_str()
        },
        if controller.ui.browser.search_busy {
            " | filtering…"
        } else {
            ""
        }
    );
    let right = status_bar_right_text(selected_column, controller.average_fps());
    StatusBarModel {
        left,
        center,
        right,
    }
}

fn status_bar_right_text(selected_column: usize, avg_fps: Option<f64>) -> String {
    match avg_fps {
        Some(avg_fps) => format!("fps: {avg_fps:.1} | col: {}/3", selected_column + 1),
        None => format!("col: {}/3", selected_column + 1),
    }
}

pub(crate) fn selected_column_index(ui: &UiState) -> usize {
    ui.browser
        .selected
        .map(|selected| match TriageFlagColumn::from(selected.column) {
            TriageFlagColumn::Trash => 0,
            TriageFlagColumn::Neutral => 1,
            TriageFlagColumn::Keep => 2,
        })
        .unwrap_or(1)
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
            can_clear_recovery_log: !ui.sources.folders.delete_recovery.entries.is_empty()
                && !ui.sources.folders.delete_recovery.in_progress,
        },
        folder_recovery: FolderRecoveryModel {
            in_progress: ui.sources.folders.delete_recovery.in_progress,
            entry_count: ui.sources.folders.delete_recovery.entries.len(),
        },
    }
}

fn project_browser_model(controller: &mut AppController) -> BrowserPanelModel {
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let search_placeholder = Some(String::from("Search samples (Ctrl+F)"));
    let busy = controller.ui.browser.search_busy;
    let sort_label =
        Some(browser_sort_label(SampleBrowserSort::from(controller.ui.browser.sort)).to_owned());
    let active_tab_label =
        Some(browser_tab_label(controller.ui.browser.active_tab.into()).to_owned());
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let visible_count = controller.ui.browser.visible.len();
    if controller.ui.browser.active_tab == SampleBrowserTab::Map {
        return BrowserPanelModel {
            visible_count,
            selected_visible_row,
            selected_path_count,
            search_query,
            search_placeholder,
            busy,
            sort_label,
            active_tab_label,
            focused_sample_label,
            anchor_visible_row,
            rows: Vec::new(),
        };
    }
    let mut rows = Vec::new();
    let selected_paths = if controller.ui.browser.selected_paths.is_empty() {
        None
    } else {
        Some(
            controller
                .ui
                .browser
                .selected_paths
                .iter()
                .cloned()
                .collect::<HashSet<_>>(),
        )
    };
    let (window_start, window_len) =
        browser_render_window(visible_count, selected_visible_row, anchor_visible_row);
    let mut visible_rows = Vec::with_capacity(window_len);
    controller
        .ui
        .browser
        .visible
        .copy_window_into(window_start, window_len, &mut visible_rows);
    for (offset, absolute_index) in visible_rows.into_iter().enumerate() {
        let visible_row = window_start + offset;
        let Some((entry_tag, relative_path)) = controller
            .wav_entry(absolute_index)
            .map(|entry| (entry.tag, entry.relative_path.clone()))
        else {
            let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
            rows.push(
                BrowserRowModel::new(
                    visible_row,
                    format!("row {}", visible_row + 1),
                    1,
                    false,
                    focused,
                )
                .with_bucket_label(String::from("SAMPLE")),
            );
            continue;
        };
        let label = controller.label_for_ref(absolute_index).map(str::to_string);
        let selected = selected_paths
            .as_ref()
            .is_some_and(|selected_paths| selected_paths.contains(&relative_path));
        let row_label = label.unwrap_or_else(|| view_model::sample_display_label(&relative_path));
        let bucket_label = browser_bucket_label(controller, &relative_path, entry_tag);
        let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
        rows.push(
            BrowserRowModel::new(
                visible_row,
                row_label,
                browser_column_index(entry_tag),
                selected,
                focused,
            )
            .with_bucket_label(bucket_label),
        );
    }

    BrowserPanelModel {
        visible_count,
        selected_visible_row,
        selected_path_count,
        search_query,
        search_placeholder,
        busy,
        sort_label,
        active_tab_label,
        focused_sample_label,
        anchor_visible_row,
        rows,
    }
}

fn project_browser_chrome_model(ui: &UiState, visible_count: usize) -> BrowserChromeModel {
    BrowserChromeModel {
        samples_tab_label: String::from("Samples"),
        map_tab_label: String::from("Similarity map"),
        search_prefix_label: String::from("Search"),
        search_placeholder: String::from("Search samples (Ctrl+F)"),
        activity_ready_label: String::from("Ready"),
        activity_busy_label: String::from("Filtering"),
        sort_prefix_label: String::from("Sort"),
        sort_order_label: browser_sort_label(SampleBrowserSort::from(ui.browser.sort)).to_owned(),
        similarity_toggle_label: if ui.browser.similarity_sort_follow_loaded {
            String::from("follow loaded")
        } else {
            String::from("manual anchor")
        },
        item_count_label: format!("{visible_count} items"),
    }
}

fn browser_render_window(
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> (usize, usize) {
    if visible_count == 0 {
        return (0, 0);
    }
    let window_len = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    if window_len == visible_count {
        return (0, window_len);
    }
    let pivot = selected_visible_row
        .or(anchor_visible_row)
        .unwrap_or(0)
        .min(visible_count - 1);
    let half_window = window_len / 2;
    let max_start = visible_count - window_len;
    let window_start = pivot.saturating_sub(half_window).min(max_start);
    (window_start, window_len)
}

fn project_waveform_model(controller: &mut AppController) -> WaveformPanelModel {
    let ui = &controller.ui;
    let view_span = (ui.waveform.view.end - ui.waveform.view.start).clamp(0.000_1, 1.0) as f32;
    let zoom_percent = (100.0 / view_span).round().clamp(100.0, 9999.0);
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
        tempo_label: ui.waveform.bpm_value.map(|bpm| format!("{bpm:.1} BPM")),
        zoom_label: Some(format!("{zoom_percent:.0}%")),
        waveform_image_signature: ui.waveform.waveform_image_signature,
        waveform_image: project_waveform_image(controller),
    }
}

fn project_waveform_image(
    controller: &mut AppController,
) -> Option<ImageRgba> {
    if controller.projected_waveform_image_signature == controller.ui.waveform.waveform_image_signature {
        return controller.projected_waveform_image.clone();
    }
    let projected_waveform_image = project_waveform_image_data(&controller.ui.waveform.image);
    controller.projected_waveform_image_signature = controller.ui.waveform.waveform_image_signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

fn project_waveform_image_data(
    image: &Option<crate::waveform::WaveformImage>,
) -> Option<ImageRgba> {
    let image = image.as_ref()?;
    if image.size[0] == 0 || image.size[1] == 0 {
        return None;
    }
    let mut pixels = Vec::with_capacity(image.size[0].saturating_mul(image.size[1]).saturating_mul(4));
    for pixel in &image.pixels {
        pixels.push(pixel.r());
        pixels.push(pixel.g());
        pixels.push(pixel.b());
        pixels.push(pixel.a());
    }
    ImageRgba::new(image.size[0], image.size[1], pixels)
}

fn project_waveform_chrome_model(ui: &UiState) -> WaveformChromeModel {
    WaveformChromeModel {
        transport_hint: if ui.waveform.loop_enabled {
            String::from("Loop enabled")
        } else {
            String::from("Loop disabled")
        },
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

fn browser_bucket_label(
    controller: &mut AppController,
    relative_path: &Path,
    tag: crate::sample_sources::Rating,
) -> String {
    if let Some(bpm) = controller.bpm_value_for_path(relative_path) {
        return format_bpm_badge_label(bpm);
    }
    match browser_column_index(tag) {
        0 => String::from("TRASH"),
        2 => String::from("KEEP"),
        _ => String::from("SAMPLE"),
    }
}

fn format_bpm_badge_label(bpm: f32) -> String {
    if !bpm.is_finite() || bpm <= 0.0 {
        return String::from("SAMPLE");
    }
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.05 {
        format!("{rounded:.0} BPM")
    } else {
        format!("{bpm:.1} BPM")
    }
}

fn browser_sort_label(sort: SampleBrowserSort) -> &'static str {
    match sort {
        SampleBrowserSort::ListOrder => "List order",
        SampleBrowserSort::Similarity => "Similarity",
        SampleBrowserSort::PlaybackAgeAsc => "Playback age ↑",
        SampleBrowserSort::PlaybackAgeDesc => "Playback age ↓",
    }
}

fn browser_tab_label(tab: SampleBrowserTab) -> &'static str {
    match tab {
        SampleBrowserTab::List => "Samples",
        SampleBrowserTab::Map => "Similarity map",
    }
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalize_folder_name_input(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(String::from("Folder name cannot be empty"));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(String::from("Folder name is invalid"));
    }
    if trimmed.contains(['/', '\\']) {
        return Err(String::from("Folder name cannot contain path separators"));
    }
    Ok(trimmed.to_string())
}

fn folder_with_name(target: &Path, name: &str) -> PathBuf {
    target.parent().map_or_else(
        || PathBuf::from(name),
        |parent| {
            if parent.as_os_str().is_empty() {
                PathBuf::from(name)
            } else {
                parent.join(name)
            }
        },
    )
}

fn folder_exists_in_rows(ui: &UiState, relative_path: &Path) -> bool {
    ui.sources
        .folders
        .rows
        .iter()
        .any(|row| row.path == relative_path)
}

fn folder_create_validation_error(ui: &UiState, parent: &Path, name: &str) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let relative = if parent.as_os_str().is_empty() {
        PathBuf::from(&normalized)
    } else {
        parent.join(&normalized)
    };
    folder_exists_in_rows(ui, &relative)
        .then_some(format!("Folder already exists: {}", relative.display()))
}

fn folder_rename_validation_error(ui: &UiState, target: &Path, name: &str) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let renamed = folder_with_name(target, &normalized);
    if renamed == target {
        return None;
    }
    folder_exists_in_rows(ui, &renamed)
        .then_some(format!("Folder already exists: {}", renamed.display()))
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
    fn browser_render_window_limits_to_target_size() {
        let (start, len) = browser_render_window(500, None, None);
        assert_eq!(start, 0);
        assert_eq!(len, 500);
    }

    #[test]
    fn browser_render_window_centers_on_selected_row() {
        let (start, len) = browser_render_window(500, Some(250), None);
        assert_eq!(len, 500);
        assert_eq!(start, 0);
    }

    #[test]
    fn browser_render_window_clamps_near_end_of_visible_rows() {
        let (start, len) = browser_render_window(500, Some(490), None);
        assert_eq!(len, 500);
        assert_eq!(start, 0);
    }

    #[test]
    fn browser_render_window_limits_large_visible_sets_to_cap() {
        let (start, len) = browser_render_window(1_200, None, None);
        assert_eq!(start, 0);
        assert_eq!(len, MAX_RENDERED_BROWSER_ROWS);
    }

    #[test]
    fn browser_render_window_centers_and_tail_clamps_for_large_visible_sets() {
        let (center_start, center_len) = browser_render_window(1_200, Some(800), None);
        assert_eq!(center_len, MAX_RENDERED_BROWSER_ROWS);
        assert_eq!(center_start, 544);

        let (tail_start, tail_len) = browser_render_window(1_200, Some(1_190), None);
        assert_eq!(tail_len, MAX_RENDERED_BROWSER_ROWS);
        assert_eq!(tail_start, 688);
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
    fn browser_projection_exposes_sort_tab_and_search_hint_labels() {
        let mut controller =
            AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
        controller.ui.browser.sort = SampleBrowserSort::PlaybackAgeDesc.into();
        controller.ui.browser.active_tab = SampleBrowserTab::Map.into();
        controller.ui.browser.visible = crate::app::state::VisibleRows::All { total: 42 };
        let projected = project_browser_model(&mut controller);
        assert_eq!(
            projected.search_placeholder.as_deref(),
            Some("Search samples (Ctrl+F)")
        );
        assert_eq!(projected.sort_label.as_deref(), Some("Playback age ↓"));
        assert_eq!(
            projected.active_tab_label.as_deref(),
            Some("Similarity map")
        );
        assert!(projected.rows.is_empty());
        assert_eq!(projected.visible_count, 42);
    }

    #[test]
    fn browser_chrome_projection_exposes_toolbar_and_tab_copy() {
        let mut ui = UiState::default();
        ui.browser.sort = SampleBrowserSort::Similarity.into();
        ui.browser.similarity_sort_follow_loaded = true;
        let projected = project_browser_chrome_model(&ui, 1437);
        assert_eq!(projected.samples_tab_label, "Samples");
        assert_eq!(projected.map_tab_label, "Similarity map");
        assert_eq!(projected.search_prefix_label, "Search");
        assert_eq!(projected.search_placeholder, "Search samples (Ctrl+F)");
        assert_eq!(projected.activity_ready_label, "Ready");
        assert_eq!(projected.activity_busy_label, "Filtering");
        assert_eq!(projected.sort_prefix_label, "Sort");
        assert_eq!(projected.sort_order_label, "Similarity");
        assert_eq!(projected.similarity_toggle_label, "follow loaded");
        assert_eq!(projected.item_count_label, "1437 items");
    }

    #[test]
    fn waveform_projection_exposes_tempo_and_zoom_labels() {
        let mut ui = UiState::default();
        ui.waveform.bpm_value = Some(128.0);
        ui.waveform.view.start = 0.25;
        ui.waveform.view.end = 0.75;
        let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
        controller.ui.waveform.bpm_value = Some(128.0);
        controller.ui.waveform.view.start = 0.25;
        controller.ui.waveform.view.end = 0.75;
        let projected = project_waveform_model(&mut controller);
        assert_eq!(projected.tempo_label.as_deref(), Some("128.0 BPM"));
        assert_eq!(projected.zoom_label.as_deref(), Some("200%"));
        assert!(projected.waveform_image.is_none());
    }

    #[test]
    fn waveform_projection_passes_raster_image_payload() {
        let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
        controller.ui.waveform.image = Some(crate::waveform::WaveformImage {
            size: [2, 1],
            pixels: vec![
                crate::waveform::WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 40),
                crate::waveform::WaveformRgba::from_rgba_unmultiplied(11, 21, 31, 41),
            ],
        });
        let projected = project_waveform_model(&mut controller);
        let waveform_image = projected
            .waveform_image
            .as_ref()
            .expect("waveform image should be projected");
        assert_eq!(waveform_image.width, 2);
        assert_eq!(waveform_image.height, 1);
        assert_eq!(
            waveform_image.pixels,
            vec![10, 20, 30, 40, 11, 21, 31, 41]
        );
    }

    #[test]
    fn waveform_chrome_projection_reflects_loop_hint() {
        let mut ui = UiState::default();
        ui.waveform.loop_enabled = false;
        let projected = project_waveform_chrome_model(&ui);
        assert_eq!(projected.transport_hint, "Loop disabled");

        ui.waveform.loop_enabled = true;
        let projected = project_waveform_chrome_model(&ui);
        assert_eq!(projected.transport_hint, "Loop enabled");
    }

    #[test]
    fn update_projection_exposes_status_and_action_hint_labels() {
        let mut ui = UiState::default();
        let projected = project_update_model(&ui);
        assert_eq!(projected.status, UpdateStatusModel::Idle);
        assert_eq!(projected.status_label, "Updates: idle");
        assert_eq!(projected.action_hint_label, "Action: check");
        assert!(projected.release_notes_label.is_empty());

        ui.update.status = UpdateStatus::UpdateAvailable.into();
        ui.update.available_tag = Some(String::from("v20.1.0"));
        ui.update.available_url = Some(String::from("https://example.invalid/release"));
        ui.update.available_published_at = Some(String::from("2026-02-01T12:00:00Z"));
        let projected = project_update_model(&ui);
        assert_eq!(projected.status, UpdateStatusModel::Available);
        assert_eq!(projected.status_label, "Update available: v20.1.0");
        assert_eq!(
            projected.action_hint_label,
            "Actions: open | install | dismiss"
        );
        assert_eq!(
            projected.release_notes_label,
            "Release: v20.1.0 (2026-02-01T12:00:00Z)"
        );

        ui.update.status = UpdateStatus::Error.into();
        ui.update.last_error = Some(String::from("network timeout"));
        let projected = project_update_model(&ui);
        assert_eq!(projected.status, UpdateStatusModel::Error);
        assert_eq!(
            projected.status_label,
            "Update check failed: network timeout"
        );
        assert_eq!(projected.action_hint_label, "Action: retry");
        assert!(projected.release_notes_label.is_empty());
    }

    #[test]
    fn map_projection_exposes_legend_selection_and_viewport_labels() {
        let mut controller =
            AppController::new(crate::waveform::WaveformRenderer::new(32, 32), None);
        controller.ui.browser.active_tab = SampleBrowserTab::Map.into();
        controller.ui.map.zoom = 1.75;
        controller.ui.map.pan.x = 12.0;
        controller.ui.map.pan.y = -8.0;
        controller.ui.map.selected_sample_id = Some(String::from("source::kick_24.wav"));
        controller.ui.map.hovered_sample_id = Some(String::from("source::kick_hover.wav"));

        let projected = project_map_model(&mut controller);
        assert!(projected.active);
        assert!(projected.legend_label.starts_with("Render:"));
        assert!(projected.selection_label.contains("Selection:"));
        assert!(projected.hover_label.contains("Hover:"));
        assert!(projected.cluster_label.starts_with("Clusters:"));
        assert_eq!(projected.viewport_label, "zoom 1.75x | pan (12, -8)");
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
        ui.browser.pending_action = Some(
            SampleBrowserActionPrompt::Rename {
                target: std::path::PathBuf::from("kick.wav"),
                name: String::from("kick"),
            }
            .into(),
        );
        ui.waveform.pending_destructive = Some(
            DestructiveEditPrompt {
                edit: DestructiveSelectionEdit::TrimSelection,
                title: String::from("Trim selection"),
                message: String::from("Apply trim?"),
            }
            .into(),
        );
        let projected = project_confirm_prompt_model(&ui);
        assert!(projected.visible);
        assert_eq!(projected.kind, Some(ConfirmPromptKind::BrowserRename));
    }

    #[test]
    fn confirm_prompt_projects_folder_create_inline_state() {
        let mut ui = UiState::default();
        ui.sources.folders.new_folder = Some(
            InlineFolderCreation {
                parent: std::path::PathBuf::from("drums"),
                name: String::from("kicks"),
                focus_requested: true,
            }
            .into(),
        );
        let projected = project_confirm_prompt_model(&ui);
        assert!(projected.visible);
        assert_eq!(projected.kind, Some(ConfirmPromptKind::FolderCreate));
        assert_eq!(projected.confirm_label, "Create");
        assert_eq!(projected.input_value.as_deref(), Some("kicks"));
        assert_eq!(
            projected.input_placeholder.as_deref(),
            Some("New folder name")
        );
    }

    #[test]
    fn confirm_prompt_projects_folder_create_validation_errors() {
        let mut ui = UiState::default();
        ui.sources.folders.rows.push(
            FolderRowView {
                path: std::path::PathBuf::from("drums/existing"),
                name: String::from("existing"),
                depth: 1,
                has_children: false,
                expanded: false,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: false,
                root_filter_mode: None,
            }
            .into(),
        );
        ui.sources.folders.new_folder = Some(
            InlineFolderCreation {
                parent: std::path::PathBuf::from("drums"),
                name: String::from("existing"),
                focus_requested: true,
            }
            .into(),
        );
        let projected = project_confirm_prompt_model(&ui);
        assert_eq!(
            projected.input_error.as_deref(),
            Some("Folder already exists: drums/existing")
        );

        if let Some(new_folder) = ui.sources.folders.new_folder.as_mut() {
            new_folder.name = String::from("bad/name");
        }
        let projected = project_confirm_prompt_model(&ui);
        assert_eq!(
            projected.input_error.as_deref(),
            Some("Folder name cannot contain path separators")
        );
    }

    #[test]
    fn confirm_prompt_projects_folder_rename_validation_errors() {
        let mut ui = UiState::default();
        ui.sources.folders.rows.push(
            FolderRowView {
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
            }
            .into(),
        );
        ui.sources.folders.rows.push(
            FolderRowView {
                path: std::path::PathBuf::from("kicks"),
                name: String::from("kicks"),
                depth: 1,
                has_children: false,
                expanded: false,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: false,
                root_filter_mode: None,
            }
            .into(),
        );
        ui.sources.folders.pending_action = Some(
            FolderActionPrompt::Rename {
                target: std::path::PathBuf::from("drums"),
                name: String::from("kicks"),
            }
            .into(),
        );
        let projected = project_confirm_prompt_model(&ui);
        assert_eq!(
            projected.input_error.as_deref(),
            Some("Folder already exists: kicks")
        );

        ui.sources.folders.pending_action = Some(
            FolderActionPrompt::Rename {
                target: std::path::PathBuf::from("drums"),
                name: String::from("../bad"),
            }
            .into(),
        );
        let projected = project_confirm_prompt_model(&ui);
        assert_eq!(
            projected.input_error.as_deref(),
            Some("Folder name cannot contain path separators")
        );
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
        ui.sources.folders.rows.push(
            FolderRowView {
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
            }
            .into(),
        );
        ui.sources.folders.focused = Some(0);
        let projected = project_sources_model(&ui);
        assert!(projected.folder_actions.can_create_folder);
        assert!(projected.folder_actions.can_create_folder_at_root);
        assert!(!projected.folder_actions.can_rename_folder);
        assert!(!projected.folder_actions.can_delete_folder);

        ui.sources.folders.rows.push(
            FolderRowView {
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
            }
            .into(),
        );
        ui.sources.folders.focused = Some(1);
        let projected = project_sources_model(&ui);
        assert!(projected.folder_actions.can_rename_folder);
        assert!(projected.folder_actions.can_delete_folder);
    }

    #[test]
    fn folder_actions_disable_recovery_clear_while_recovery_is_running() {
        let mut ui = UiState::default();
        ui.sources.folders.delete_recovery.entries.push(
            FolderDeleteRecoveryEntry {
                source_label: String::from("source"),
                relative_path: std::path::PathBuf::from("drums"),
                action: FolderDeleteRecoveryAction::Restore,
                status: FolderDeleteRecoveryStatus::Completed,
                detail: None,
            }
            .into(),
        );
        ui.sources.folders.delete_recovery.in_progress = true;
        let projected = project_sources_model(&ui);
        assert!(!projected.folder_actions.can_clear_recovery_log);
    }

    #[test]
    fn status_bar_right_text_includes_fps_when_present() {
        assert_eq!(status_bar_right_text(0, Some(59.9)), "fps: 59.9 | col: 1/3");
    }

    #[test]
    fn status_bar_right_text_defaults_to_column_only_without_fps() {
        assert_eq!(status_bar_right_text(2, None), "col: 3/3");
    }
}
