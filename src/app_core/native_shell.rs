//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use super::controller::{
    AppController, ProjectedBrowserPreloadWindow, ProjectedBrowserRowCacheEntry,
    ProjectedMapPointCacheEntry, ProjectedMapPointsCacheKey, ProjectedSelectedPathsLookup,
    UmapPointQuery,
};
use crate::app_core::actions::NativeFolderRowKind as FolderRowKind;
use crate::app_core::actions::{
    NativeAppModel as AppModel, NativeBrowserActionsModel as BrowserActionsModel,
    NativeBrowserChromeModel as BrowserChromeModel, NativeBrowserPanelModel as BrowserPanelModel,
    NativeBrowserRowModel as BrowserRowModel, NativeColumnModel as ColumnModel,
    NativeConfirmPromptKind as ConfirmPromptKind, NativeConfirmPromptModel as ConfirmPromptModel,
    NativeDragOverlayModel as DragOverlayModel, NativeFocusContextModel as FocusContextModel,
    NativeFolderActionsModel as FolderActionsModel,
    NativeFolderRecoveryModel as FolderRecoveryModel, NativeFolderRowModel as FolderRowModel,
    NativeMapPanelModel as MapPanelModel, NativeMapRenderModeModel as MapRenderModeModel,
    NativeMotionModel as MotionModel, NativeNormalizedRangeModel as NormalizedRangeModel,
    NativeOptionsPanelModel as OptionsPanelModel,
    NativeProgressOverlayModel as ProgressOverlayModel, NativeSourceRowModel as SourceRowModel,
    NativeSourcesPanelModel as SourcesPanelModel, NativeStatusBarModel as StatusBarModel,
    NativeUpdatePanelModel as UpdatePanelModel, NativeUpdateStatusModel as UpdateStatusModel,
    NativeWaveformChromeModel as WaveformChromeModel,
    NativeWaveformPanelModel as WaveformPanelModel,
};
use crate::app_core::app_api::state::DragPayload;
use crate::app_core::app_api::state::FocusContext;
#[cfg(test)]
use crate::app_core::state::{
    DestructiveEditPrompt, DestructiveSelectionEdit, FolderDeleteRecoveryAction,
    FolderDeleteRecoveryEntry, FolderDeleteRecoveryStatus, FolderRowView, InlineFolderEdit,
};
use crate::app_core::state::{
    DragTarget, FolderActionPrompt, InlineFolderEditKind, MapBounds, MapPoint, MapQueryBounds,
    MapRenderMode, SampleBrowserActionPrompt, SampleBrowserSort, SampleBrowserTab,
    TriageFlagColumn, UiState, UpdateStatus,
};
use crate::app_core::ui::{MAX_RENDERED_BROWSER_ROWS, MAX_RENDERED_MAP_POINTS};
use crate::gui::types::ImageRgba;
use crate::{analysis::similarity::SIMILARITY_MODEL_ID, app_core::view_model};
use std::{
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::info;

/// Staged top-level app-model projection assembly.
mod app_model;
/// Browser panel/frame/row projection helpers and retained browser caches.
mod browser_projection;
/// Confirm-prompt projection helpers and folder-name validation utilities.
mod confirm_prompt_projection;
/// Map panel projection helpers and retained projected map-point caches.
mod map_projection;
/// Options-panel projection helpers.
mod options_panel_projection;
/// Source/folder sidebar projection helpers.
mod sources_projection;
/// Status-bar and selected-column projection helpers.
mod status_projection;
/// Update panel projection helpers.
mod update_projection;
/// Shared waveform-image translation helpers used by controller and projection paths.
mod waveform_image_translation;
/// Waveform panel and waveform chrome projection helpers.
mod waveform_projection;

pub(crate) use app_model::project_app_model;
#[cfg(test)]
use app_model::{
    assemble_project_app_model, derive_project_app_model_inputs,
    materialize_project_app_model_core, materialize_project_app_model_overlay_and_chrome,
};
#[cfg(test)]
use browser_projection::{
    browser_bpm_preload_ranges, browser_column_index, browser_render_window,
    browser_row_identity_hash, project_cached_browser_row, refresh_projected_browser_row_cache,
    refresh_projected_selected_paths_lookup, selected_index_is_selected,
};
pub(crate) use browser_projection::{
    project_browser_chrome_model, project_browser_model, project_browser_panel_frame_model,
    project_browser_rows_model_into, project_browser_rows_projection_inputs,
};
pub(crate) use confirm_prompt_projection::project_confirm_prompt_model;
pub(crate) use map_projection::project_map_model;
pub(crate) use options_panel_projection::project_options_panel_model;
pub(crate) use sources_projection::project_sources_model;
use status_projection::status_bar_right_text;
pub(crate) use status_projection::{project_status_model, selected_column_index};
pub(crate) use update_projection::project_update_model;
pub(crate) use waveform_image_translation::waveform_image_to_native_rgba;
pub(crate) use waveform_projection::{project_waveform_chrome_model, project_waveform_model};

static PROJECT_APP_MODEL_CALLS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of browser-row cache lookups that reused retained row projection data.
static BROWSER_ROW_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "native-bridge-metrics")]
/// Number of browser-row cache lookups that rebuilt retained row projection data.
static BROWSER_ROW_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
/// Cap retained browser-row projection cache growth per selected source.
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

/// Project app focus context into the native runtime focus model.
pub(crate) fn project_focus_context_model(focus: FocusContext) -> FocusContextModel {
    match focus {
        FocusContext::None => FocusContextModel::None,
        FocusContext::Waveform => FocusContextModel::Waveform,
        FocusContext::SampleBrowser => FocusContextModel::SampleBrowser,
        FocusContext::SourceFolders => FocusContextModel::SourceFolders,
        FocusContext::SourcesList => FocusContextModel::SourcesList,
    }
}

/// Project motion-only model fields used by animation-phase redraws.
///
/// This path intentionally avoids rebuilding static panel payloads and should
/// stay aligned with corresponding waveform/map/status fields in `project_app_model`.
pub(crate) fn project_motion_model(controller: &mut AppController) -> MotionModel {
    let selected_column = selected_column_index(&controller.ui);
    let fade_overlay =
        waveform_projection::project_waveform_edit_fade_overlay_model(&controller.ui);
    let projected_playhead = waveform_projection::projected_playhead_ratio(controller);
    MotionModel {
        transport_running: controller.is_playing(),
        map_active: matches!(
            SampleBrowserTab::from(controller.ui.browser.active_tab),
            SampleBrowserTab::Map
        ),
        active_rating_filters: {
            let mut flags = [false; 8];
            for (index, level) in [-3, -2, -1, 0, 1, 2, 3, 4].into_iter().enumerate() {
                flags[index] = controller.ui.browser.search.rating_filter.contains(&level);
            }
            flags
        },
        waveform_selection_milli: controller.ui.waveform.selection.map(|selection| {
            crate::app_core::actions::NativeNormalizedRangeModel::from_micros(
                waveform_projection::normalized_to_micros(selection.start()),
                waveform_projection::normalized_to_micros(selection.end()),
            )
        }),
        waveform_slices: waveform_projection::project_waveform_slice_previews(&controller.ui),
        waveform_selection_export_flash_nonce: controller.ui.waveform.selection_export_flash_nonce,
        waveform_selection_export_failure_flash_nonce: controller
            .ui
            .waveform
            .selection_export_failure_flash_nonce,
        waveform_edit_selection_apply_flash_nonce: controller
            .ui
            .waveform
            .edit_selection_apply_flash_nonce,
        waveform_edit_selection_milli: waveform_projection::project_waveform_edit_selection_milli(
            &controller.ui,
        ),
        waveform_edit_fade_in_end_milli: fade_overlay.fade_in_end_milli,
        waveform_edit_fade_in_end_micros: fade_overlay.fade_in_end_micros,
        waveform_edit_fade_in_mute_start_milli: fade_overlay.fade_in_mute_start_milli,
        waveform_edit_fade_in_mute_start_micros: fade_overlay.fade_in_mute_start_micros,
        waveform_edit_fade_in_curve_milli: fade_overlay.fade_in_curve_milli,
        waveform_edit_fade_out_start_milli: fade_overlay.fade_out_start_milli,
        waveform_edit_fade_out_start_micros: fade_overlay.fade_out_start_micros,
        waveform_edit_fade_out_mute_end_milli: fade_overlay.fade_out_mute_end_milli,
        waveform_edit_fade_out_mute_end_micros: fade_overlay.fade_out_mute_end_micros,
        waveform_edit_fade_out_curve_milli: fade_overlay.fade_out_curve_milli,
        waveform_loop_enabled: controller.ui.waveform.loop_enabled,
        waveform_cursor_milli: controller
            .ui
            .waveform
            .cursor
            .map(waveform_projection::normalized_to_milli),
        waveform_playhead_milli: projected_playhead.map(waveform_projection::normalized_to_milli),
        waveform_playhead_micros: projected_playhead.map(waveform_projection::normalized_to_micros),
        waveform_view_start_milli: waveform_projection::normalized64_to_milli(
            controller.ui.waveform.view.start,
        ),
        waveform_view_end_milli: waveform_projection::normalized64_to_milli(
            controller.ui.waveform.view.end,
        ),
        waveform_view_start_micros: waveform_projection::normalized64_to_micros(
            controller.ui.waveform.view.start,
        ),
        waveform_view_end_micros: waveform_projection::normalized64_to_micros(
            controller.ui.waveform.view.end,
        ),
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
        waveform_loading: controller.ui.waveform.loading.is_some(),
        waveform_transport_hint: if controller.ui.waveform.loop_enabled {
            String::from("Loop enabled")
        } else {
            String::from("Loop disabled")
        },
        waveform_channel_view: waveform_projection::project_waveform_channel_view_model(
            controller.ui.waveform.channel_view,
        ),
        waveform_normalized_audition_enabled: controller.ui.waveform.normalized_audition_enabled,
        waveform_bpm_snap_enabled: controller.ui.waveform.bpm_snap_enabled,
        waveform_relative_bpm_grid_enabled: controller.ui.waveform.relative_bpm_grid_enabled,
        waveform_transient_snap_enabled: controller.ui.waveform.transient_snap_enabled,
        waveform_transient_markers_enabled: controller.ui.waveform.transient_markers_enabled,
        waveform_slice_mode_enabled: controller.ui.waveform.slice_mode_enabled,
        status_right: status_bar_right_text(selected_column),
    }
}

/// Project browser action availability for toolbar command enablement.
pub(crate) fn project_browser_actions_model(ui: &UiState) -> BrowserActionsModel {
    let has_focus = ui.browser.selection.selected_visible.is_some();
    let has_selection = has_focus || !ui.browser.selection.selected_paths.is_empty();
    BrowserActionsModel {
        can_rename: has_focus,
        can_delete: has_selection,
        can_tag: has_selection,
        random_navigation_enabled: ui.browser.search.random_navigation_mode,
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

/// Project drag-overlay feedback content for active drag sessions.
pub(crate) fn project_drag_overlay_model(ui: &UiState) -> DragOverlayModel {
    let active = ui.drag.payload.is_some();
    if !active {
        return DragOverlayModel::default();
    }
    let active_target = DragTarget::from(ui.drag.active_target.clone());
    let target_label = match &active_target {
        DragTarget::None => String::from("No target"),
        DragTarget::BrowserList => String::from("Sample list"),
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
    let valid_target = match (ui.drag.payload.as_ref(), &active_target) {
        (
            Some(
                DragPayload::Sample { .. }
                | DragPayload::Samples { .. }
                | DragPayload::Folder { .. }
                | DragPayload::DropTargetReorder { .. },
            ),
            DragTarget::BrowserList,
        ) => false,
        _ => !matches!(active_target, DragTarget::None),
    };
    let (pointer_x, pointer_y) = ui
        .drag
        .position
        .map(native_drag_overlay_pointer_anchor)
        .unwrap_or((None, None));
    DragOverlayModel {
        active,
        label: ui.drag.label.clone(),
        target_label,
        valid_target,
        pointer_x,
        pointer_y,
    }
}

/// Convert the retained drag cursor position into native overlay anchor coordinates.
fn native_drag_overlay_pointer_anchor(
    position: crate::app::state::UiPoint,
) -> (Option<u16>, Option<u16>) {
    (
        native_drag_overlay_pointer_component(position.x),
        native_drag_overlay_pointer_component(position.y),
    )
}

/// Clamp one floating drag-chip coordinate into the native overlay wire format.
fn native_drag_overlay_pointer_component(value: f32) -> Option<u16> {
    if !value.is_finite() {
        return None;
    }
    Some(value.round().clamp(0.0, f32::from(u16::MAX)) as u16)
}

#[cfg(test)]
/// Unit tests for native-shell projection and retained cache behavior.
mod tests;
