//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use super::controller::{
    AppController, ProjectedBrowserRowCacheEntry, ProjectedMapPointCacheEntry,
    ProjectedMapPointsCacheKey, ProjectedSelectedPathsLookup, UmapPointQuery,
};
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
#[cfg(test)]
use crate::app_core::state::{
    DestructiveEditPrompt, DestructiveSelectionEdit, FolderDeleteRecoveryAction,
    FolderDeleteRecoveryEntry, FolderDeleteRecoveryStatus, FolderRowView, InlineFolderCreation,
};
use crate::app_core::state::{
    DragTarget, FolderActionPrompt, MapBounds, MapPoint, MapQueryBounds, MapRenderMode,
    SampleBrowserActionPrompt, SampleBrowserSort, SampleBrowserTab, TriageFlagColumn, UiState,
    UpdateStatus,
};
use crate::app_core::ui::{MAX_RENDERED_BROWSER_ROWS, MAX_RENDERED_MAP_POINTS};
use crate::gui::types::ImageRgba;
use crate::{analysis::similarity::SIMILARITY_MODEL_ID, app_core::view_model};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::info;

/// Browser panel/frame/row projection helpers and retained browser caches.
mod browser_projection;
/// Map panel projection helpers and retained projected map-point caches.
mod map_projection;
/// Waveform panel and waveform chrome projection helpers.
mod waveform_projection;

#[cfg(test)]
use browser_projection::{
    browser_render_window, browser_row_identity_hash, project_cached_browser_row,
    refresh_projected_browser_row_cache, refresh_projected_selected_paths_lookup,
    selected_index_is_selected,
};
pub(crate) use browser_projection::{
    project_browser_chrome_model, project_browser_model, project_browser_panel_frame_model,
    project_browser_rows_model_into,
};
pub(crate) use map_projection::project_map_model;
pub(crate) use waveform_projection::{project_waveform_chrome_model, project_waveform_model};

static PROJECT_APP_MODEL_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of browser-row cache lookups that reused retained row projection data.
static BROWSER_ROW_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of browser-row cache lookups that rebuilt retained row projection data.
static BROWSER_ROW_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
/// Cap retained browser-row projection cache growth per visible-row revision.
const MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE: usize = MAX_RENDERED_BROWSER_ROWS * 8;

/// Record one browser-row cache hit/miss lookup decision.
#[cfg(feature = "native-bridge-metrics")]
fn trace_browser_row_cache_lookup(hit: bool) {
    if hit {
        BROWSER_ROW_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        BROWSER_ROW_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

/// No-op browser-row cache lookup tracer when bridge metrics are disabled.
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_browser_row_cache_lookup(_hit: bool) {}

/// Return browser-row cache hit/miss counters for bridge profiling summaries.
#[cfg(feature = "native-bridge-metrics")]
pub(crate) fn browser_row_cache_lookup_counts() -> (u64, u64) {
    (
        BROWSER_ROW_CACHE_HIT_COUNT.load(Ordering::Relaxed),
        BROWSER_ROW_CACHE_MISS_COUNT.load(Ordering::Relaxed),
    )
}

pub(crate) fn project_app_model(controller: &mut AppController) -> AppModel {
    let call = PROJECT_APP_MODEL_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    let derived_inputs = derive_project_app_model_inputs(controller);
    if call <= 12 {
        info!(
            call,
            selected_column = derived_inputs.selected_column,
            status_len = derived_inputs.status_text.len(),
            visible_browser_rows = controller.ui.browser.visible.len(),
            "native shell: project_app_model start"
        );
    }
    let core_models = materialize_project_app_model_core(controller, &derived_inputs);
    let overlay_and_chrome_models = materialize_project_app_model_overlay_and_chrome(
        &controller.ui,
        core_models.browser.visible_count,
    );
    let app_model =
        assemble_project_app_model(derived_inputs, core_models, overlay_and_chrome_models);
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

/// Immutable projection inputs derived once per app-model projection.
struct ProjectAppModelDerivedInputs {
    /// Selected triage column index used by status and top-level model metadata.
    selected_column: usize,
    /// Transport-running state projected into the top-level app model.
    transport_running: bool,
    /// Flat status text mirrored in the top-level app model.
    status_text: String,
    /// Triage/browser item counts used to project top-level column metadata.
    column_counts: [usize; 3],
    /// Master output volume clamped into the normalized `0.0..=1.0` range.
    clamped_volume: f32,
}

/// Core panel models that may require mutable controller access during projection.
struct ProjectAppModelCoreModels {
    /// Source and folder panel model.
    sources: SourcesPanelModel,
    /// Status bar segment model.
    status: StatusBarModel,
    /// Browser action availability model.
    browser_actions: BrowserActionsModel,
    /// Map panel model.
    map: MapPanelModel,
    /// Waveform panel model.
    waveform: WaveformPanelModel,
    /// Browser panel model (frame metadata + row window).
    browser: BrowserPanelModel,
}

/// Overlay and chrome models that depend on projected core model metadata.
struct ProjectAppModelOverlayAndChromeModels {
    /// Update surface model.
    update: UpdatePanelModel,
    /// Progress overlay model.
    progress_overlay: ProgressOverlayModel,
    /// Confirm prompt overlay model.
    confirm_prompt: ConfirmPromptModel,
    /// Drag feedback overlay model.
    drag_overlay: DragOverlayModel,
    /// Browser chrome/toolbar labels.
    browser_chrome: BrowserChromeModel,
    /// Waveform header chrome labels.
    waveform_chrome: WaveformChromeModel,
}

/// Derive scalar projection inputs shared across staged app-model materialization.
fn derive_project_app_model_inputs(controller: &AppController) -> ProjectAppModelDerivedInputs {
    ProjectAppModelDerivedInputs {
        selected_column: selected_column_index(&controller.ui),
        transport_running: controller.is_playing(),
        status_text: controller.ui.status.text.clone(),
        column_counts: [
            controller.ui.browser.trash.len(),
            controller.ui.browser.neutral.len(),
            controller.ui.browser.keep.len(),
        ],
        clamped_volume: controller.ui.volume.clamp(0.0, 1.0),
    }
}

/// Materialize core panel models for the staged app-model projection pipeline.
fn materialize_project_app_model_core(
    controller: &mut AppController,
    derived_inputs: &ProjectAppModelDerivedInputs,
) -> ProjectAppModelCoreModels {
    ProjectAppModelCoreModels {
        sources: project_sources_model(&controller.ui),
        status: project_status_model(controller, derived_inputs.selected_column),
        browser_actions: project_browser_actions_model(&controller.ui),
        map: project_map_model(controller),
        waveform: project_waveform_model(controller),
        browser: project_browser_model(controller),
    }
}

/// Materialize overlays/chrome after core panels are projected.
fn materialize_project_app_model_overlay_and_chrome(
    ui: &UiState,
    browser_visible_count: usize,
) -> ProjectAppModelOverlayAndChromeModels {
    ProjectAppModelOverlayAndChromeModels {
        update: project_update_model(ui),
        progress_overlay: project_progress_overlay_model(ui),
        confirm_prompt: project_confirm_prompt_model(ui),
        drag_overlay: project_drag_overlay_model(ui),
        browser_chrome: project_browser_chrome_model(ui, browser_visible_count),
        waveform_chrome: project_waveform_chrome_model(ui),
    }
}

/// Assemble the final native app model from staged projection outputs.
fn assemble_project_app_model(
    derived_inputs: ProjectAppModelDerivedInputs,
    core_models: ProjectAppModelCoreModels,
    overlay_and_chrome_models: ProjectAppModelOverlayAndChromeModels,
) -> AppModel {
    AppModel {
        title: String::from("Sempal"),
        backend_label: String::from("backend: native_vello"),
        sources_label: format!("Sources ({})", core_models.sources.rows.len()),
        status_text: derived_inputs.status_text,
        status: core_models.status,
        browser_actions: core_models.browser_actions,
        progress_overlay: overlay_and_chrome_models.progress_overlay,
        confirm_prompt: overlay_and_chrome_models.confirm_prompt,
        drag_overlay: overlay_and_chrome_models.drag_overlay,
        columns: [
            ColumnModel::new("Trash", derived_inputs.column_counts[0]),
            ColumnModel::new("Samples", derived_inputs.column_counts[1]),
            ColumnModel::new("Keep", derived_inputs.column_counts[2]),
        ],
        selected_column: derived_inputs.selected_column,
        volume: derived_inputs.clamped_volume,
        transport_running: derived_inputs.transport_running,
        sources: core_models.sources,
        browser: core_models.browser,
        browser_chrome: overlay_and_chrome_models.browser_chrome,
        map: core_models.map,
        waveform: core_models.waveform,
        waveform_chrome: overlay_and_chrome_models.waveform_chrome,
        update: overlay_and_chrome_models.update,
    }
}

pub(crate) fn project_motion_model(controller: &mut AppController) -> MotionModel {
    let selected_column = selected_column_index(&controller.ui);
    let status = project_status_model(controller, selected_column);
    let (edit_fade_in_end_milli, edit_fade_out_start_milli) = controller
        .ui
        .waveform
        .edit_selection
        .map(project_motion_edit_fade_handles_milli)
        .unwrap_or((None, None));
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
        waveform_edit_selection_milli: controller.ui.waveform.edit_selection.map(|selection| {
            crate::app_core::actions::NativeNormalizedRangeModel::new(
                normalized_to_milli(selection.start()),
                normalized_to_milli(selection.end()),
            )
        }),
        waveform_edit_fade_in_end_milli: edit_fade_in_end_milli,
        waveform_edit_fade_out_start_milli: edit_fade_out_start_milli,
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
        waveform_channel_view: match controller.ui.waveform.channel_view {
            crate::waveform::WaveformChannelView::Mono => {
                radiant::app::WaveformChannelViewModel::Mono
            }
            crate::waveform::WaveformChannelView::SplitStereo => {
                radiant::app::WaveformChannelViewModel::Stereo
            }
        },
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        status_right: status.right,
    }
}

/// Project edit-fade handles for motion-only waveform overlay updates.
fn project_motion_edit_fade_handles_milli(
    selection: crate::selection::SelectionRange,
) -> (Option<u16>, Option<u16>) {
    let start = selection.start();
    let end = selection.end();
    let width = selection.width();
    if width <= 0.0 {
        return (None, None);
    }
    let fade_in_end_milli = selection.fade_in().map(|fade| {
        let fade_end = (start + (width * fade.length).max(0.0)).clamp(start, end);
        normalized_to_milli(fade_end)
    });
    let fade_out_start_milli = selection.fade_out().map(|fade| {
        let fade_start = (end - (width * fade.length).max(0.0)).clamp(start, end);
        normalized_to_milli(fade_start)
    });
    (fade_in_end_milli, fade_out_start_milli)
}

/// Project update panel state into the native shell model.
pub(crate) fn project_update_model(ui: &UiState) -> UpdatePanelModel {
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
            .map(|tag| format!("Update available: {tag} (manual install required)"))
            .unwrap_or_else(|| String::from("Update available (manual install required)")),
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
                String::from("Actions: open | install(manual) | dismiss")
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
                format!("Release: {tag} ({published_at}) | Signed manual install required")
            } else {
                format!("Release: {tag} | Signed manual install required")
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

/// Project map panel state into the native shell model.
pub(crate) fn project_browser_actions_model(ui: &UiState) -> BrowserActionsModel {
    let has_focus = ui.browser.selected_visible.is_some();
    let has_selection = has_focus || !ui.browser.selected_paths.is_empty();
    BrowserActionsModel {
        can_rename: has_focus,
        can_delete: has_selection,
        can_tag: has_selection,
    }
}

/// Project progress-overlay state for native runtime rendering.
pub(crate) fn project_progress_overlay_model(ui: &UiState) -> ProgressOverlayModel {
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

/// Project active confirm prompt metadata for modal rendering.
pub(crate) fn project_confirm_prompt_model(ui: &UiState) -> ConfirmPromptModel {
    if let Some(SampleBrowserActionPrompt::Rename { target, name }) =
        ui.browser.pending_action.clone()
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
    if let Some(FolderActionPrompt::Rename { target, name }) =
        ui.sources.folders.pending_action.clone()
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
    if let Some(prompt) = ui.waveform.pending_destructive.clone() {
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

/// Project drag-overlay feedback content for active drag sessions.
pub(crate) fn project_drag_overlay_model(ui: &UiState) -> DragOverlayModel {
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

/// Project status-bar text segments for the native shell footer.
pub(crate) fn project_status_model(
    controller: &AppController,
    selected_column: usize,
) -> StatusBarModel {
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
    let right = status_bar_right_text(selected_column);
    StatusBarModel {
        left,
        center,
        right,
    }
}

fn status_bar_right_text(selected_column: usize) -> String {
    format!("col: {}/3", selected_column + 1)
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

/// Project source/folder panel data for the native sidebar.
pub(crate) fn project_sources_model(ui: &UiState) -> SourcesPanelModel {
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
/// Unit tests for native-shell projection and retained cache behavior.
mod tests;
