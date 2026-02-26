//! Native-shell projection helpers used by the `radiant` bridge.
//!
//! The bridge consumes these helpers to project controller state into
//! backend-neutral `radiant::app` models and to translate normalized UI ranges
//! back into controller-domain selection math.

use super::controller::{
    AppController, ProjectedBrowserRowCacheEntry, ProjectedMapPointCacheEntry,
    ProjectedMapPointsCacheKey,
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
pub(crate) fn project_map_model(controller: &mut AppController) -> MapPanelModel {
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
    let source_id_key = source_id.as_ref().map(|id| id.as_str().to_string());
    let umap_version = controller.ui.map.umap_version.clone();
    let has_matching_bounds_cache = controller.ui.map.cached_bounds_source_id == source_id_key
        && controller.ui.map.cached_bounds_umap_version.as_deref() == Some(umap_version.as_str());
    let bounds = if has_matching_bounds_cache {
        controller.ui.map.bounds
    } else {
        match controller.umap_bounds(SIMILARITY_MODEL_ID, &umap_version, source_id.as_ref()) {
            Ok(bounds) => {
                let mapped_bounds = bounds.map(|value| MapBounds {
                    min_x: value.min_x,
                    max_x: value.max_x,
                    min_y: value.min_y,
                    max_y: value.max_y,
                });
                controller.ui.map.cached_bounds_source_id = source_id_key.clone();
                controller.ui.map.cached_bounds_umap_version = Some(umap_version.clone());
                controller.ui.map.bounds = mapped_bounds;
                mapped_bounds
            }
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
    let query_bounds = MapQueryBounds {
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y,
        max_y: bounds.max_y,
    };
    let has_matching_points_cache = controller.ui.map.cached_points_source_id == source_id_key
        && controller.ui.map.cached_points_umap_version.as_deref() == Some(umap_version.as_str())
        && controller.ui.map.last_query == Some(query_bounds);
    if !has_matching_points_cache {
        match controller.umap_points_in_bounds(
            SIMILARITY_MODEL_ID,
            &umap_version,
            "umap",
            &umap_version,
            source_id.as_ref(),
            query_bounds,
            MAX_RENDERED_MAP_POINTS,
        ) {
            Ok(points) => {
                controller.ui.map.cached_points = points
                    .iter()
                    .map(|point| MapPoint {
                        sample_id: point.sample_id.clone(),
                        x: point.x,
                        y: point.y,
                        cluster_id: point.cluster_id,
                    })
                    .collect::<Vec<_>>();
                controller.ui.map.cached_points_source_id = source_id_key.clone();
                controller.ui.map.cached_points_umap_version = Some(umap_version.clone());
                controller.ui.map.last_query = Some(query_bounds);
                controller.ui.map.cached_points_revision =
                    controller.ui.map.cached_points_revision.saturating_add(1);
            }
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
        }
    }

    let focused_sample_id = controller.selected_sample_id();
    let selected_sample_id = controller.ui.map.selected_sample_id.clone();
    let projection_key = map_projection_cache_key(
        source_id_key.as_deref(),
        umap_version.as_str(),
        controller.ui.map.cached_points_revision,
        query_bounds,
    );
    refresh_projected_map_points_cache(controller, projection_key, bounds);
    let cluster_count = controller.projected_map_cluster_count;
    let points = project_map_points_model(
        controller.projected_map_points.as_slice(),
        selected_sample_id.as_deref(),
        focused_sample_id.as_deref(),
    );
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

/// Build a retained map-projection cache key from active map source/query state.
fn map_projection_cache_key(
    source_id: Option<&str>,
    umap_version: &str,
    points_revision: u64,
    query_bounds: MapQueryBounds,
) -> ProjectedMapPointsCacheKey {
    ProjectedMapPointsCacheKey {
        source_id_hash: hash_scalar(source_id.unwrap_or_default()),
        umap_version_hash: hash_scalar(umap_version),
        points_revision,
        query_min_x_bits: query_bounds.min_x.to_bits(),
        query_max_x_bits: query_bounds.max_x.to_bits(),
        query_min_y_bits: query_bounds.min_y.to_bits(),
        query_max_y_bits: query_bounds.max_y.to_bits(),
    }
}

/// Refresh retained normalized map-point cache only when projection key changes.
fn refresh_projected_map_points_cache(
    controller: &mut AppController,
    key: ProjectedMapPointsCacheKey,
    bounds: MapBounds,
) {
    if controller.projected_map_points_key == Some(key) {
        return;
    }
    let (projected_points, cluster_count) = {
        let points = controller.ui.map.cached_points.as_slice();
        build_projected_map_points_cache(bounds, points)
    };
    controller.projected_map_points_key = Some(key);
    controller.projected_map_points = projected_points;
    controller.projected_map_cluster_count = cluster_count;
}

/// Build normalized map-point cache entries and unique cluster summary in one pass.
fn build_projected_map_points_cache(
    bounds: MapBounds,
    points: &[MapPoint],
) -> (Vec<ProjectedMapPointCacheEntry>, usize) {
    let denom_x = (bounds.max_x - bounds.min_x).max(1e-6);
    let denom_y = (bounds.max_y - bounds.min_y).max(1e-6);
    let mut cluster_ids = HashSet::new();
    let mut projected_points = Vec::with_capacity(points.len());
    for point in points {
        if let Some(cluster_id) = point.cluster_id {
            cluster_ids.insert(cluster_id);
        }
        let x = ((point.x - bounds.min_x) / denom_x).clamp(0.0, 1.0);
        let y = ((point.y - bounds.min_y) / denom_y).clamp(0.0, 1.0);
        projected_points.push(ProjectedMapPointCacheEntry {
            sample_id: point.sample_id.clone(),
            x_milli: normalized_to_milli(x),
            y_milli: normalized_to_milli(y),
            cluster_id: point.cluster_id,
        });
    }
    (projected_points, cluster_ids.len())
}

/// Project final map points by applying dynamic selected/focused state flags.
fn project_map_points_model(
    projected_points: &[ProjectedMapPointCacheEntry],
    selected_sample_id: Option<&str>,
    focused_sample_id: Option<&str>,
) -> Vec<MapPointModel> {
    let mut points = Vec::with_capacity(projected_points.len());
    for point in projected_points {
        points.push(MapPointModel {
            sample_id: point.sample_id.clone(),
            x_milli: point.x_milli,
            y_milli: point.y_milli,
            cluster_id: point.cluster_id,
            selected: selected_sample_id.is_some_and(|selected| selected == point.sample_id),
            focused: focused_sample_id.is_some_and(|focused| focused == point.sample_id),
        });
    }
    points
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

/// Project browser action availability for native action surfaces.
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

/// Project browser metadata that excludes the rendered row vector.
///
/// This is split from row projection so callers can retain stable frame fields
/// and refresh only row content when needed.
pub(crate) fn project_browser_panel_frame_model(controller: &AppController) -> BrowserPanelModel {
    let selected_visible_row = controller.ui.browser.selected_visible;
    let selected_path_count = controller.ui.browser.selected_paths.len();
    let search_query = controller.ui.browser.search_query.clone();
    let search_placeholder = Some(String::from("Search samples (Ctrl+F)"));
    let busy = controller.ui.browser.search_busy;
    let sort_label =
        Some(browser_sort_label(SampleBrowserSort::from(controller.ui.browser.sort)).to_owned());
    let active_tab_label = Some(browser_tab_label(controller.ui.browser.active_tab).to_owned());
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    let anchor_visible_row = controller.ui.browser.selection_anchor_visible;
    let visible_count = controller.ui.browser.visible.len();
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
        rows: Vec::new(),
    }
}

/// Project browser row content for the current visible window.
///
/// This helper is intentionally separated from metadata projection so callers
/// can refresh row content independently of browser header/search/tab state.
pub(crate) fn project_browser_rows_model(
    controller: &mut AppController,
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> Vec<BrowserRowModel> {
    let mut rows = Vec::with_capacity(visible_count.min(MAX_RENDERED_BROWSER_ROWS));
    project_browser_rows_model_into(
        controller,
        visible_count,
        selected_visible_row,
        anchor_visible_row,
        &mut rows,
    );
    rows
}

/// Project browser row content into an existing row-model buffer.
///
/// Callers that retain `rows` across frames can reuse vector capacity to
/// reduce allocation churn in high-frequency browser projection paths.
pub(crate) fn project_browser_rows_model_into(
    controller: &mut AppController,
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    rows: &mut Vec<BrowserRowModel>,
) {
    if controller.ui.browser.active_tab == SampleBrowserTab::Map {
        clear_projected_browser_row_cache(controller);
        clear_projected_selected_paths_lookup(controller);
        rows.clear();
        return;
    }
    refresh_projected_browser_row_cache(controller);
    refresh_projected_selected_paths_lookup(controller);
    let (window_start, window_len) =
        browser_render_window(visible_count, selected_visible_row, anchor_visible_row);
    preload_browser_window_bpms(controller, window_start, window_len);
    if rows.capacity() < window_len {
        rows.reserve(window_len.saturating_sub(rows.len()));
    }
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.visible.get(visible_row) else {
            continue;
        };
        let Some((cached_row, selected)) = project_cached_browser_row(controller, absolute_index)
        else {
            let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
            write_browser_row_into_slot(
                rows,
                offset,
                (
                    visible_row,
                    &format!("row {}", visible_row + 1),
                    1,
                    "SAMPLE",
                    false,
                    focused,
                ),
            );
            continue;
        };
        let focused = selected_visible_row.is_some_and(|focused| focused == visible_row);
        write_browser_row_into_slot(
            rows,
            offset,
            (
                visible_row,
                &cached_row.row_label,
                cached_row.column_index,
                &cached_row.bucket_label,
                selected,
                focused,
            ),
        );
    }
    rows.truncate(window_len);
}

/// Preload BPM metadata for the current visible browser window in one batch query.
fn preload_browser_window_bpms(
    controller: &mut AppController,
    window_start: usize,
    window_len: usize,
) {
    if window_len == 0 {
        return;
    }
    let mut visible_paths = Vec::with_capacity(window_len);
    for offset in 0..window_len {
        let visible_row = window_start + offset;
        let Some(absolute_index) = controller.ui.browser.visible.get(visible_row) else {
            continue;
        };
        if let Some(relative_path) = controller
            .wav_entry(absolute_index)
            .map(|entry| entry.relative_path.clone())
        {
            visible_paths.push(relative_path);
        }
    }
    controller.preload_bpm_values_for_paths(&visible_paths);
}

/// Project browser panel metadata and row window into one panel model.
pub(crate) fn project_browser_model(controller: &mut AppController) -> BrowserPanelModel {
    let mut panel = project_browser_panel_frame_model(controller);
    panel.rows = project_browser_rows_model(
        controller,
        panel.visible_count,
        panel.selected_visible_row,
        panel.anchor_visible_row,
    );
    panel
}

/// Hash a scalar key into one stable 64-bit cache key.
fn hash_scalar<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash one relative path into a stable row-identity scalar.
fn browser_row_identity_hash(path: &Path) -> u64 {
    hash_scalar(path)
}

/// Clear the retained selected-path lookup cache.
fn clear_projected_selected_paths_lookup(controller: &mut AppController) {
    controller.projected_selected_paths_revision =
        Some(controller.ui.browser.selected_paths_revision);
    controller.projected_selected_paths_lookup = None;
}

/// Refresh the retained selected-index bitset when selection changes.
fn refresh_projected_selected_paths_lookup(controller: &mut AppController) {
    let selection_revision = controller.ui.browser.selected_paths_revision;
    if controller.ui.browser.selected_paths.is_empty() {
        if controller.projected_selected_paths_lookup.is_some()
            || controller.projected_selected_paths_revision != Some(selection_revision)
        {
            clear_projected_selected_paths_lookup(controller);
        }
        return;
    }
    if controller.projected_selected_paths_revision == Some(selection_revision)
        && controller.projected_selected_paths_lookup.is_some()
    {
        return;
    }
    let mut selected_index_lookup = vec![false; controller.wav_entries_len()];
    for selected_path_idx in 0..controller.ui.browser.selected_paths.len() {
        let selected_path = controller.ui.browser.selected_paths[selected_path_idx].clone();
        if let Some(absolute_index) = controller.wav_index_for_path(selected_path.as_path())
            && let Some(selected) = selected_index_lookup.get_mut(absolute_index)
        {
            *selected = true;
        }
    }
    controller.projected_selected_paths_revision = Some(selection_revision);
    controller.projected_selected_paths_lookup = Some(selected_index_lookup);
}

/// Return whether one absolute row index is selected in the retained lookup bitset.
fn selected_index_is_selected(controller: &AppController, absolute_index: usize) -> bool {
    controller
        .projected_selected_paths_lookup
        .as_ref()
        .and_then(|lookup| lookup.get(absolute_index))
        .copied()
        .unwrap_or(false)
}

/// Clear retained browser-row projection fields.
fn clear_projected_browser_row_cache(controller: &mut AppController) {
    controller.projected_browser_rows.clear();
}

/// Reset retained browser-row projection fields when visible rows changed materially.
fn refresh_projected_browser_row_cache(controller: &mut AppController) {
    if controller.projected_browser_rows_revision == controller.ui.browser.visible_rows_revision {
        return;
    }
    controller.projected_browser_rows_revision = controller.ui.browser.visible_rows_revision;
    clear_projected_browser_row_cache(controller);
}

/// Return true when one cached browser-row projection still matches the entry snapshot.
fn cached_browser_row_matches_entry(
    cached: &ProjectedBrowserRowCacheEntry,
    row_identity_hash: u64,
    column_index: usize,
) -> bool {
    cached.row_identity_hash == row_identity_hash && cached.column_index == column_index
}

/// Resolve static browser-row projection fields from cache, inserting on cache miss.
fn project_cached_browser_row(
    controller: &mut AppController,
    absolute_index: usize,
) -> Option<(&ProjectedBrowserRowCacheEntry, bool)> {
    let (entry_tag, row_identity_hash) = controller.wav_entry(absolute_index).map(|entry| {
        (
            entry.tag,
            browser_row_identity_hash(entry.relative_path.as_path()),
        )
    })?;
    let column_index = browser_column_index(entry_tag);
    let cache_hit = controller
        .projected_browser_rows
        .get(&absolute_index)
        .is_some_and(|cached| {
            cached_browser_row_matches_entry(cached, row_identity_hash, column_index)
        });
    trace_browser_row_cache_lookup(cache_hit);
    if !cache_hit {
        let relative_path = controller
            .wav_entry(absolute_index)
            .map(|entry| entry.relative_path.clone())?;
        let row_label = controller
            .label_for_ref(absolute_index)
            .map(str::to_string)
            .unwrap_or_else(|| view_model::sample_display_label(relative_path.as_path()));
        let cached = ProjectedBrowserRowCacheEntry {
            row_identity_hash,
            row_label,
            column_index,
            bucket_label: browser_bucket_label(controller, relative_path.as_path(), entry_tag),
        };
        if controller.projected_browser_rows.len() >= MAX_RETAINED_BROWSER_ROW_PROJECTION_CACHE {
            clear_projected_browser_row_cache(controller);
        }
        controller
            .projected_browser_rows
            .insert(absolute_index, cached);
    }
    let projected = controller.projected_browser_rows.get(&absolute_index)?;
    Some((
        projected,
        selected_index_is_selected(controller, absolute_index),
    ))
}

/// Write one browser row into `rows[offset]`, reusing existing `String` buffers.
fn write_browser_row_into_slot(
    rows: &mut Vec<BrowserRowModel>,
    offset: usize,
    projection: (usize, &str, usize, &str, bool, bool),
) {
    let (visible_row, row_label, column_index, bucket_label, selected, focused) = projection;
    if let Some(row) = rows.get_mut(offset) {
        row.visible_row = visible_row;
        row.label.clear();
        row.label.push_str(row_label);
        row.column = column_index.min(2);
        row.selected = selected;
        row.focused = focused;
        if let Some(existing_bucket_label) = row.bucket_label.as_mut() {
            existing_bucket_label.clear();
            existing_bucket_label.push_str(bucket_label);
        } else {
            row.bucket_label = Some(bucket_label.to_owned());
        }
        return;
    }
    rows.push(
        BrowserRowModel::new(visible_row, row_label, column_index, selected, focused)
            .with_bucket_label(bucket_label),
    );
}

/// Project browser toolbar/tab/footer labels.
pub(crate) fn project_browser_chrome_model(
    ui: &UiState,
    visible_count: usize,
) -> BrowserChromeModel {
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

/// Project waveform panel data for native waveform rendering.
pub(crate) fn project_waveform_model(controller: &mut AppController) -> WaveformPanelModel {
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

/// Reuse or rebuild the projected waveform raster payload for the native model.
fn project_waveform_image(controller: &mut AppController) -> Option<Arc<ImageRgba>> {
    let signature = controller.ui.waveform.waveform_image_signature;
    let has_source_image = controller.ui.waveform.image.is_some();
    let has_cached_image = controller.projected_waveform_image.is_some();
    if signature.is_some()
        && controller.projected_waveform_image_signature == signature
        && has_source_image == has_cached_image
    {
        return controller.projected_waveform_image.clone();
    }
    // Producer-side waveform rendering now publishes shared immutable RGBA payloads and
    // versioned identities. Keep a projection-side fallback for tests/manual image assignment.
    let projected_waveform_image = controller
        .projected_waveform_image
        .clone()
        .or_else(|| {
            controller
                .ui
                .waveform
                .image
                .as_ref()
                .and_then(
                    crate::app::controller::library::wavs::waveform_rendering::waveform_image_to_native_rgba,
                )
        });
    controller.projected_waveform_image_signature = signature;
    controller.projected_waveform_image = projected_waveform_image.clone();
    projected_waveform_image
}

/// Project waveform chrome labels and action-hint copy.
pub(crate) fn project_waveform_chrome_model(ui: &UiState) -> WaveformChromeModel {
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
/// Unit tests for native-shell projection and retained cache behavior.
mod tests;
