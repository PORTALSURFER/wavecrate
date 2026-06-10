//! UI projection helpers used by retained bridge and test contracts.
//!
//! These helpers project controller state into Wavecrate-owned DTOs and translate
//! normalized UI ranges back into controller-domain selection math. New default
//! GUI work should compose Radiant's current public API through `src/native_app.rs`
//! instead of expanding this migration projection surface.

use super::controller::{
    AppController, ProjectedBrowserPreloadWindow, ProjectedBrowserRowCacheEntry,
    ProjectedMapPointCacheEntry, ProjectedMapPointsCacheKey, ProjectedSelectedPathsLookup,
    UmapPointQuery,
};
use crate::app_core::actions::NativeFolderRowKind as FolderRowKind;
use crate::app_core::actions::{
    NativeAppModel as AppModel, NativeAudioEngineModel as AudioEngineModel,
    NativeBrowserActionsModel as BrowserActionsModel,
    NativeBrowserChromeModel as BrowserChromeModel, NativeBrowserPanelModel as BrowserPanelModel,
    NativeBrowserRowModel as BrowserRowModel, NativeBrowserTagPillModel as BrowserTagPillModel,
    NativeBrowserTagSidebarModel as BrowserTagSidebarModel,
    NativeBrowserTagState as BrowserTagState, NativeColumnModel as ColumnModel,
    NativeConfirmPromptKind as ConfirmPromptKind, NativeConfirmPromptModel as ConfirmPromptModel,
    NativeDragOverlayModel as DragOverlayModel, NativeFocusContextModel as FocusContextModel,
    NativeFolderActionsModel as FolderActionsModel, NativeFolderPaneIdModel as FolderPaneIdModel,
    NativeFolderPaneModel as FolderPaneModel, NativeFolderRecoveryModel as FolderRecoveryModel,
    NativeFolderRowModel as FolderRowModel, NativeMapPanelModel as MapPanelModel,
    NativeMapRenderModeModel as MapRenderModeModel,
    NativeNormalizedRangeModel as NormalizedRangeModel,
    NativeOptionsPanelModel as OptionsPanelModel,
    NativeProgressOverlayModel as ProgressOverlayModel, NativeRetainedVec as RetainedVec,
    NativeSourceRowModel as SourceRowModel, NativeSourcesPanelModel as SourcesPanelModel,
    NativeStatusBarModel as StatusBarModel, NativeUpdatePanelModel as UpdatePanelModel,
    NativeUpdateStatusModel as UpdateStatusModel, NativeWaveformChromeModel as WaveformChromeModel,
    NativeWaveformPanelModel as WaveformPanelModel, native_folder_row_model as folder_row_model,
};
use crate::app_core::app_api::controller::supports_wav_destructive_edits;
use crate::app_core::app_api::state::DragPayload;
use crate::app_core::app_api::state::FocusContext;
use crate::app_core::app_api::state::UiPoint;
#[cfg(test)]
use crate::app_core::state::{
    DestructiveEditPrompt, DestructiveSelectionEdit, FolderDeleteRecoveryAction,
    FolderDeleteRecoveryEntry, FolderDeleteRecoveryStatus, FolderRowView, InlineFolderEdit,
};
use crate::app_core::state::{
    DragTarget, FolderActionPrompt, InlineFolderEditKind, MapBounds, MapPoint, MapQueryBounds,
    MapRenderMode, PlaybackAgeBucket, PlaybackAgeFilterChip, SampleBrowserActionPrompt,
    SampleBrowserSort, SampleBrowserTab, TriageFlagColumn, UiState, UpdateStatus,
};
use crate::app_core::ui::{MAX_RENDERED_BROWSER_ROWS, MAX_RENDERED_MAP_POINTS};
use crate::ui_primitives::types::ImageRgba;
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
/// Motion-only projection helpers used by paint/animation updates.
mod motion_projection;
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
    patch_browser_rows_state, project_browser_chrome_model, project_browser_focused_sample_label,
    project_browser_model, project_browser_panel_frame_model, project_browser_rows_model_into,
    project_browser_rows_projection_inputs, project_browser_tag_sidebar_model,
};
pub(crate) use confirm_prompt_projection::project_confirm_prompt_model;
pub(crate) use map_projection::project_map_model;
pub(crate) use motion_projection::project_motion_model;
pub(crate) use options_panel_projection::{
    project_audio_engine_chip_model, project_audio_engine_model, project_options_panel_model,
};
pub(crate) use sources_projection::project_sources_model;
use status_projection::status_bar_right_text;
pub(crate) use status_projection::{project_status_model, selected_column_index};
pub(crate) use update_projection::project_update_model;
pub(crate) use waveform_image_translation::waveform_image_to_native_rgba;
pub(crate) use waveform_projection::{
    effective_waveform_image_signature, project_waveform_chrome_model, project_waveform_model,
};

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

/// Project app focus context into the UI runtime focus model.
pub(crate) fn project_focus_context_model(focus: FocusContext) -> FocusContextModel {
    match focus {
        FocusContext::None => FocusContextModel::None,
        FocusContext::Waveform => FocusContextModel::Waveform,
        FocusContext::SampleBrowser => FocusContextModel::SampleBrowser,
        FocusContext::SourceFolders => FocusContextModel::SourceFolders,
        FocusContext::SourcesList => FocusContextModel::SourcesList,
    }
}

/// Project browser action availability for toolbar command enablement.
pub(crate) fn project_browser_actions_model(ui: &UiState) -> BrowserActionsModel {
    let has_focus = ui.browser.selection.selected_visible.is_some();
    let has_selection = has_focus || !ui.browser.selection.selected_paths.is_empty();
    let focused_path = ui.browser.selection.selected_paths.first().or(ui
        .browser
        .selection
        .last_focused_path
        .as_ref());
    let can_apply_wav_destructive_edit =
        focused_path.is_some_and(|path| supports_wav_destructive_edits(path));
    BrowserActionsModel {
        can_rename: has_focus,
        can_delete: has_selection,
        can_tag: has_selection,
        can_normalize_focused_sample: has_focus && can_apply_wav_destructive_edit,
        can_loop_crossfade_focused_sample: has_focus && can_apply_wav_destructive_edit,
        random_navigation_enabled: ui.browser.search.random_navigation_mode,
        duplicate_cleanup_active: ui.browser.duplicate_cleanup.is_some(),
        tag_sidebar_open: ui.browser.tag_sidebar_open
            && matches!(ui.browser.active_tab, SampleBrowserTab::List),
    }
}

/// Project progress-overlay state for UI runtime rendering.
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
        DragTarget::FolderPanel { folder, .. } => folder
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

/// Convert the retained drag cursor position into UI overlay anchor coordinates.
fn native_drag_overlay_pointer_anchor(position: UiPoint) -> (Option<u16>, Option<u16>) {
    (
        native_drag_overlay_pointer_component(position.x),
        native_drag_overlay_pointer_component(position.y),
    )
}

/// Clamp one floating drag-chip coordinate into the UI overlay wire format.
fn native_drag_overlay_pointer_component(value: f32) -> Option<u16> {
    if !value.is_finite() {
        return None;
    }
    Some(value.round().clamp(0.0, f32::from(u16::MAX)) as u16)
}

#[cfg(test)]
/// Unit tests for ui-projection and retained cache behavior.
mod tests;
