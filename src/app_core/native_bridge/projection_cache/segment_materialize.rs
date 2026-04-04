use super::{
    DerivedProjectionState, NativeProjectionCache, ProjectionSegment, trace_projection_cache_lookup,
};
use crate::app_core::actions::{NativeAppModel, NativeBrowserPanelModel, NativeDirtySegments};
use crate::app_core::controller::AppController;
use crate::app_core::native_shell;
use std::sync::Arc;

/// Resolve the retained app-model snapshot using a fresh derive-state snapshot.
pub(super) fn resolve_or_project(
    cache: &mut NativeProjectionCache,
    controller: &mut AppController,
) -> (Arc<NativeAppModel>, NativeDirtySegments) {
    let _ = controller.refresh_projection_revision_bus();
    let derived = DerivedProjectionState::from_controller(controller);
    resolve_or_project_with_derived(cache, controller, &derived)
}

/// Resolve retained projection output using a caller-provided derive state.
pub(super) fn resolve_or_project_with_derived(
    cache: &mut NativeProjectionCache,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
) -> (Arc<NativeAppModel>, NativeDirtySegments) {
    if cache.app_key.as_ref() == Some(&derived.app_key)
        && let Some(model) = cache.app_model.as_ref().map(Arc::clone)
    {
        trace_projection_cache_lookup(true);
        cache.record_segment_lookup(ProjectionSegment::StatusBar, true);
        cache.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
        cache.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
        cache.record_segment_lookup(ProjectionSegment::MapPanel, true);
        cache.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
        return (model, NativeDirtySegments::empty());
    }
    trace_projection_cache_lookup(false);
    let has_retained_model = cache.app_model.is_some();
    let mut snapshot = cache
        .app_model
        .take()
        .unwrap_or_else(|| Arc::new(NativeAppModel::default()));
    let model = Arc::make_mut(&mut snapshot);

    let mut dirty_segments = NativeDirtySegments::empty();

    if materialize_status_segment(cache, model, controller, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::STATUS_BAR);
    }

    let browser_frame_changed =
        materialize_browser_frame_segment(cache, model, controller, derived, has_retained_model);
    if browser_frame_changed {
        dirty_segments.insert(NativeDirtySegments::BROWSER_FRAME);
    }

    if materialize_browser_rows_segment(cache, model, controller, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::BROWSER_ROWS_WINDOW);
    }

    if materialize_map_segment(cache, model, controller, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::MAP_PANEL);
    }

    if materialize_waveform_segment(cache, model, controller, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::WAVEFORM_OVERLAY);
    }

    let non_segment_static_changed =
        update_non_segment_static_key(cache, derived, has_retained_model);
    if non_segment_static_changed {
        dirty_segments.insert(NativeDirtySegments::GLOBAL_STATIC);
        refresh_non_segment_static_fields(model, controller);
    }

    refresh_non_segment_always_fields(model, derived.selected_column);
    refresh_non_segment_overlay_fields(model, controller);
    cache.app_key = Some(derived.app_key.clone());
    cache.app_model = Some(Arc::clone(&snapshot));
    (snapshot, dirty_segments)
}

/// Copy browser metadata fields while preserving any retained row vector.
fn apply_browser_frame(model: &mut NativeAppModel, frame: NativeBrowserPanelModel) {
    model.browser.visible_count = frame.visible_count;
    model.browser.selected_visible_row = frame.selected_visible_row;
    model.browser.autoscroll = frame.autoscroll;
    model.browser.view_start_row = frame.view_start_row;
    model.browser.selected_path_count = frame.selected_path_count;
    model.browser.search_query = frame.search_query;
    model.browser.active_rating_filters = frame.active_rating_filters;
    model.browser.active_playback_age_filters = frame.active_playback_age_filters;
    model.browser.marked_filter_active = frame.marked_filter_active;
    model.browser.search_placeholder = frame.search_placeholder;
    model.browser.busy = frame.busy;
    model.browser.similarity_filtered = frame.similarity_filtered;
    model.browser.duplicate_cleanup_active = frame.duplicate_cleanup_active;
    model.browser.sort_label = frame.sort_label;
    model.browser.active_tab_label = frame.active_tab_label;
    model.browser.focused_sample_label = frame.focused_sample_label;
    model.browser.anchor_visible_row = frame.anchor_visible_row;
}

/// Refresh always-on non-segment metadata that is not covered by static keys.
fn refresh_non_segment_always_fields(model: &mut NativeAppModel, selected_column: usize) {
    model.selected_column = selected_column;
}

/// Refresh static non-segment app-model fields from current controller state.
fn refresh_non_segment_static_fields(model: &mut NativeAppModel, controller: &mut AppController) {
    model.transport_running = controller.is_playing();
    model.volume = controller.ui.volume.clamp(0.0, 1.0);
    model.sources = native_shell::project_sources_model(controller);
    model.sources_label = format!("Sources ({})", model.sources.rows.len());
    model.focus_context = native_shell::project_focus_context_model(controller.ui.focus.context);
    model.columns = [
        crate::app_core::actions::NativeColumnModel::new(
            "Trash",
            controller.ui.browser.trash.len(),
        ),
        crate::app_core::actions::NativeColumnModel::new(
            "Samples",
            controller.ui.browser.neutral.len(),
        ),
        crate::app_core::actions::NativeColumnModel::new("Keep", controller.ui.browser.keep.len()),
    ];
    model.update = native_shell::project_update_model(&controller.ui);
}

/// Refresh transient non-segment overlays from current controller state.
fn refresh_non_segment_overlay_fields(model: &mut NativeAppModel, controller: &AppController) {
    model.options_panel = native_shell::project_options_panel_model(&controller.ui);
    model.progress_overlay = native_shell::project_progress_overlay_model(&controller.ui);
    model.confirm_prompt = native_shell::project_confirm_prompt_model(&controller.ui);
    model.drag_overlay = native_shell::project_drag_overlay_model(&controller.ui);
}

/// Return `true` when one segment key needs rematerialization.
fn segment_key_changed<T: PartialEq>(
    has_retained_model: bool,
    cached_key: &Option<T>,
    next_key: &T,
) -> bool {
    !has_retained_model || cached_key.as_ref() != Some(next_key)
}

/// Materialize status/footer fields when the status segment is dirty.
fn materialize_status_segment(
    cache: &mut NativeProjectionCache,
    model: &mut NativeAppModel,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(has_retained_model, &cache.status_key, &derived.status_key);
    if changed {
        cache.record_segment_lookup(ProjectionSegment::StatusBar, false);
        model.status = native_shell::project_status_model(controller, derived.selected_column);
        model.status_text = controller.ui.status.text.clone();
        cache.status_key = Some(derived.status_key.clone());
    } else {
        cache.record_segment_lookup(ProjectionSegment::StatusBar, true);
    }
    changed
}

/// Materialize browser frame/chrome/action fields when the frame segment is dirty.
fn materialize_browser_frame_segment(
    cache: &mut NativeProjectionCache,
    model: &mut NativeAppModel,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.browser_frame_key,
        &derived.browser_frame_key,
    );
    if changed {
        cache.record_segment_lookup(ProjectionSegment::BrowserFrame, false);
        let frame = native_shell::project_browser_panel_frame_model(controller);
        apply_browser_frame(model, frame);
        model.browser_chrome =
            native_shell::project_browser_chrome_model(&controller.ui, model.browser.visible_count);
        model.browser_actions = native_shell::project_browser_actions_model(&controller.ui);
        cache.browser_frame_key = Some(derived.browser_frame_key.clone());
    } else {
        cache.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
    }
    changed
}

/// Materialize browser visible-row window when row-window inputs changed.
fn materialize_browser_rows_segment(
    cache: &mut NativeProjectionCache,
    model: &mut NativeAppModel,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let browser_rows_changed = segment_key_changed(
        has_retained_model,
        &cache.browser_rows_key,
        &derived.browser_rows_key,
    );
    let browser_row_state_changed = segment_key_changed(
        has_retained_model,
        &cache.browser_rows_state_key,
        &derived.browser_rows_state_key,
    );
    if browser_rows_changed {
        cache.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
        let row_inputs = native_shell::project_browser_rows_projection_inputs(controller);
        let mut rows = std::mem::take(&mut model.browser.rows);
        native_shell::project_browser_rows_model_into(
            controller,
            row_inputs.visible_count,
            row_inputs.selected_visible_row,
            row_inputs.anchor_visible_row,
            &mut rows,
        );
        model.browser.rows = rows;
        cache.browser_rows_key = Some(derived.browser_rows_key.clone());
        cache.browser_rows_state_key = Some(derived.browser_rows_state_key.clone());
        return true;
    }
    if browser_row_state_changed {
        cache.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, false);
        native_shell::patch_browser_rows_state(
            controller,
            derived.browser_rows_state_key.browser_selected_visible,
            &mut model.browser.rows,
        );
        cache.browser_rows_state_key = Some(derived.browser_rows_state_key.clone());
        return true;
    }
    cache.record_segment_lookup(ProjectionSegment::BrowserRowsWindow, true);
    false
}

/// Materialize map-panel fields when the map segment is dirty.
fn materialize_map_segment(
    cache: &mut NativeProjectionCache,
    model: &mut NativeAppModel,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(has_retained_model, &cache.map_key, &derived.map_key);
    if changed {
        cache.record_segment_lookup(ProjectionSegment::MapPanel, false);
        model.map = native_shell::project_map_model(controller);
        cache.map_key = Some(derived.map_key.clone());
    } else {
        cache.record_segment_lookup(ProjectionSegment::MapPanel, true);
    }
    changed
}

/// Materialize waveform panel/chrome fields when the waveform segment is dirty.
fn materialize_waveform_segment(
    cache: &mut NativeProjectionCache,
    model: &mut NativeAppModel,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.waveform_key,
        &derived.waveform_key,
    );
    if changed {
        cache.record_segment_lookup(ProjectionSegment::WaveformOverlay, false);
        model.waveform = native_shell::project_waveform_model(controller);
        model.waveform_chrome = native_shell::project_waveform_chrome_model(&controller.ui);
        cache.waveform_key = Some(derived.waveform_key.clone());
    } else {
        cache.record_segment_lookup(ProjectionSegment::WaveformOverlay, true);
    }
    changed
}

/// Update static non-segment cache key and report whether it changed.
fn update_non_segment_static_key(
    cache: &mut NativeProjectionCache,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.non_segment_static_key,
        &derived.non_segment_static_key,
    );
    cache.non_segment_static_key = Some(derived.non_segment_static_key.clone());
    changed
}
