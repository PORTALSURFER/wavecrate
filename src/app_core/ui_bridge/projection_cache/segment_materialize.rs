use super::{
    DerivedProjectionState, ProjectionSegment, UiProjectionCache, trace_projection_cache_lookup,
};
use crate::app_core::actions::{NativeAppModel, NativeBrowserPanelModel, NativeDirtySegments};
use crate::app_core::controller::AppController;
use crate::app_core::ui_projection;
use std::sync::Arc;

type SegmentMaterializer = for<'ctx> fn(&mut SegmentMaterializeContext<'ctx>) -> bool;

#[derive(Clone, Copy)]
struct ProjectionSegmentHandler {
    segment: ProjectionSegment,
    dirty_bits: u16,
    materialize: SegmentMaterializer,
}

struct SegmentMaterializeContext<'a> {
    cache: &'a mut UiProjectionCache,
    model: &'a mut NativeAppModel,
    controller: &'a mut AppController,
    derived: &'a DerivedProjectionState,
    has_retained_model: bool,
}

const RETAINED_SEGMENT_HANDLERS: &[ProjectionSegmentHandler] = &[
    ProjectionSegmentHandler {
        segment: ProjectionSegment::StatusBar,
        dirty_bits: NativeDirtySegments::STATUS_BAR,
        materialize: materialize_status_segment,
    },
    ProjectionSegmentHandler {
        segment: ProjectionSegment::BrowserFrame,
        dirty_bits: NativeDirtySegments::BROWSER_FRAME,
        materialize: materialize_browser_frame_segment,
    },
    ProjectionSegmentHandler {
        segment: ProjectionSegment::BrowserTagSidebar,
        dirty_bits: NativeDirtySegments::BROWSER_FRAME,
        materialize: materialize_browser_tag_sidebar_segment,
    },
    ProjectionSegmentHandler {
        segment: ProjectionSegment::BrowserRowsWindow,
        dirty_bits: NativeDirtySegments::BROWSER_ROWS_WINDOW,
        materialize: materialize_browser_rows_segment,
    },
    ProjectionSegmentHandler {
        segment: ProjectionSegment::MapPanel,
        dirty_bits: NativeDirtySegments::MAP_PANEL,
        materialize: materialize_map_segment,
    },
    ProjectionSegmentHandler {
        segment: ProjectionSegment::WaveformOverlay,
        dirty_bits: NativeDirtySegments::WAVEFORM_OVERLAY,
        materialize: materialize_waveform_segment,
    },
];

#[cfg(test)]
pub(crate) fn retained_segment_handler_plan() -> Vec<(ProjectionSegment, u16)> {
    RETAINED_SEGMENT_HANDLERS
        .iter()
        .map(|handler| (handler.segment, handler.dirty_bits))
        .collect()
}

/// Resolve the retained app-model snapshot using a fresh derive-state snapshot.
pub(super) fn resolve_or_project(
    cache: &mut UiProjectionCache,
    controller: &mut AppController,
) -> (Arc<NativeAppModel>, NativeDirtySegments) {
    let _ = controller.refresh_projection_revision_bus();
    let derived = DerivedProjectionState::from_controller(controller);
    resolve_or_project_with_derived(cache, controller, &derived)
}

/// Resolve retained projection output using a caller-provided derive state.
pub(super) fn resolve_or_project_with_derived(
    cache: &mut UiProjectionCache,
    controller: &mut AppController,
    derived: &DerivedProjectionState,
) -> (Arc<NativeAppModel>, NativeDirtySegments) {
    if cache.app_key.as_ref() == Some(&derived.app_key)
        && let Some(model) = cache.app_model.as_ref().map(Arc::clone)
    {
        trace_projection_cache_lookup(true);
        cache.record_segment_lookup(ProjectionSegment::StatusBar, true);
        cache.record_segment_lookup(ProjectionSegment::BrowserFrame, true);
        cache.record_segment_lookup(ProjectionSegment::BrowserTagSidebar, true);
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

    {
        let mut segment_context = SegmentMaterializeContext {
            cache,
            model,
            controller,
            derived,
            has_retained_model,
        };
        materialize_retained_segments(&mut segment_context, &mut dirty_segments);
    }

    let non_segment_static_changed =
        update_non_segment_static_key(cache, derived, has_retained_model);
    if non_segment_static_changed {
        dirty_segments.insert(NativeDirtySegments::GLOBAL_STATIC);
        refresh_non_segment_static_fields(model, controller);
        refresh_non_segment_audio_chip_fields(model, &controller.ui);
    }

    let non_segment_overlay_changed =
        update_non_segment_overlay_key(cache, derived, has_retained_model);
    if non_segment_overlay_changed {
        dirty_segments.insert(NativeDirtySegments::STATE_OVERLAY);
        refresh_non_segment_overlay_fields(model, controller);
    }

    refresh_non_segment_always_fields(model, derived.selected_column);
    cache.app_key = Some(derived.app_key.clone());
    cache.app_model = Some(Arc::clone(&snapshot));
    (snapshot, dirty_segments)
}

fn materialize_retained_segments(
    context: &mut SegmentMaterializeContext<'_>,
    dirty_segments: &mut NativeDirtySegments,
) {
    for handler in RETAINED_SEGMENT_HANDLERS {
        let changed = (handler.materialize)(context);
        context
            .cache
            .record_segment_lookup(handler.segment, !changed);
        if changed {
            dirty_segments.insert(handler.dirty_bits);
        }
    }
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
    model.browser.tag_named_filter_active = frame.tag_named_filter_active;
    model.browser.tag_named_filter_negated = frame.tag_named_filter_negated;
    model.browser.sidebar_filters = frame.sidebar_filters;
    model.browser.search_placeholder = frame.search_placeholder;
    model.browser.busy = frame.busy;
    model.browser.similarity_filtered = frame.similarity_filtered;
    model.browser.duplicate_cleanup_active = frame.duplicate_cleanup_active;
    model.browser.sort_label = frame.sort_label;
    model.browser.active_tab_label = frame.active_tab_label;
    model.browser.focused_sample_label = frame.focused_sample_label;
    model.browser.anchor_visible_row = frame.anchor_visible_row;
}

/// Copy tag-sidebar fields without rematerializing unrelated browser chrome.
fn apply_browser_tag_sidebar(
    model: &mut NativeAppModel,
    focused_sample_label: Option<String>,
    tag_sidebar: crate::app_core::actions::NativeBrowserTagSidebarModel,
) {
    model.browser.focused_sample_label = focused_sample_label;
    model.browser.tag_sidebar = tag_sidebar;
}

/// Refresh always-on non-segment metadata that is not covered by static keys.
fn refresh_non_segment_always_fields(model: &mut NativeAppModel, selected_column: usize) {
    model.selected_column = selected_column;
}

/// Refresh static non-segment app-model fields from current controller state.
fn refresh_non_segment_static_fields(model: &mut NativeAppModel, controller: &mut AppController) {
    model.transport_running = controller.is_playing();
    model.volume = controller.ui.volume.clamp(0.0, 1.0);
    model.sources = ui_projection::project_sources_model(controller);
    model.sources_label = format!("Sources ({})", model.sources.rows.len());
    model.focus_context = ui_projection::project_focus_context_model(controller.ui.focus.context);
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
    model.update = ui_projection::project_update_model(&controller.ui);
}

/// Refresh the lightweight audio-chip fields consumed by the static top bar.
fn refresh_non_segment_audio_chip_fields(
    model: &mut NativeAppModel,
    ui: &crate::app_core::state::UiState,
) {
    let chip = ui_projection::project_audio_engine_chip_model(ui);
    model.audio_engine.chip_state = chip.chip_state;
    model.audio_engine.chip_label = chip.chip_label;
}

/// Refresh transient non-segment overlays from current controller state.
fn refresh_non_segment_overlay_fields(model: &mut NativeAppModel, controller: &AppController) {
    let refresh_audio_engine = controller.ui.options_panel.open || model.options_panel.visible;
    if refresh_audio_engine {
        model.audio_engine = ui_projection::project_audio_engine_model(&controller.ui);
    }
    model.options_panel = ui_projection::project_options_panel_model(&controller.ui);
    model.progress_overlay = ui_projection::project_progress_overlay_model(&controller.ui);
    model.confirm_prompt = ui_projection::project_confirm_prompt_model(&controller.ui);
    model.drag_overlay = ui_projection::project_drag_overlay_model(&controller.ui);
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
fn materialize_status_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.status_key,
        &context.derived.status_key,
    );
    if changed {
        context.model.status = ui_projection::project_status_model(
            context.controller,
            context.derived.selected_column,
        );
        context.model.status_text = context.controller.ui.status.text.clone();
        context.cache.status_key = Some(context.derived.status_key.clone());
    }
    changed
}

/// Materialize browser frame/chrome/action fields when the frame segment is dirty.
fn materialize_browser_frame_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.browser_frame_key,
        &context.derived.browser_frame_key,
    );
    if changed {
        let frame = ui_projection::project_browser_panel_frame_model(context.controller);
        apply_browser_frame(context.model, frame);
        context.model.browser_chrome = ui_projection::project_browser_chrome_model(
            &context.controller.ui,
            context.model.browser.visible_count,
        );
        context.model.browser_actions =
            ui_projection::project_browser_actions_model(&context.controller.ui);
        context.cache.browser_frame_key = Some(context.derived.browser_frame_key.clone());
    }
    changed
}

/// Materialize browser tag-sidebar fields when sidebar-specific inputs changed.
fn materialize_browser_tag_sidebar_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.browser_tag_sidebar_key,
        &context.derived.browser_tag_sidebar_key,
    );
    if changed {
        apply_browser_tag_sidebar(
            context.model,
            ui_projection::project_browser_focused_sample_label(context.controller),
            ui_projection::project_browser_tag_sidebar_model(context.controller),
        );
        context.cache.browser_tag_sidebar_key =
            Some(context.derived.browser_tag_sidebar_key.clone());
    }
    changed
}

/// Materialize browser visible-row window when row-window inputs changed.
fn materialize_browser_rows_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let browser_rows_changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.browser_rows_key,
        &context.derived.browser_rows_key,
    );
    let browser_row_state_changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.browser_rows_state_key,
        &context.derived.browser_rows_state_key,
    );
    if browser_rows_changed {
        let row_inputs = ui_projection::project_browser_rows_projection_inputs(context.controller);
        ui_projection::project_browser_rows_model_into(
            context.controller,
            row_inputs.visible_count,
            row_inputs.selected_visible_row,
            row_inputs.anchor_visible_row,
            &mut context.model.browser.rows,
        );
        context.cache.browser_rows_key = Some(context.derived.browser_rows_key.clone());
        context.cache.browser_rows_state_key = Some(context.derived.browser_rows_state_key.clone());
        return true;
    }
    if browser_row_state_changed {
        ui_projection::patch_browser_rows_state(
            context.controller,
            context
                .derived
                .browser_rows_state_key
                .browser_selected_visible,
            context.model.browser.rows.make_mut().as_mut_slice(),
        );
        context.cache.browser_rows_state_key = Some(context.derived.browser_rows_state_key.clone());
        return true;
    }
    false
}

/// Materialize map-panel fields when the map segment is dirty.
fn materialize_map_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.map_key,
        &context.derived.map_key,
    );
    if changed {
        context.model.map = ui_projection::project_map_model(context.controller);
        context.cache.map_key = Some(context.derived.map_key.clone());
    }
    changed
}

/// Materialize waveform panel/chrome fields when the waveform segment is dirty.
fn materialize_waveform_segment(context: &mut SegmentMaterializeContext<'_>) -> bool {
    let changed = segment_key_changed(
        context.has_retained_model,
        &context.cache.waveform_key,
        &context.derived.waveform_key,
    );
    if changed {
        context.model.waveform = ui_projection::project_waveform_model(context.controller);
        context.model.waveform_chrome =
            ui_projection::project_waveform_chrome_model(&context.controller.ui);
        context.cache.waveform_key = Some(context.derived.waveform_key.clone());
    }
    changed
}

/// Update static non-segment cache key and report whether it changed.
fn update_non_segment_static_key(
    cache: &mut UiProjectionCache,
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

/// Update the retained non-segment overlay key and return whether it changed.
fn update_non_segment_overlay_key(
    cache: &mut UiProjectionCache,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.non_segment_overlay_key,
        &derived.non_segment_overlay_key,
    );
    cache.non_segment_overlay_key = Some(derived.non_segment_overlay_key.clone());
    changed
}
