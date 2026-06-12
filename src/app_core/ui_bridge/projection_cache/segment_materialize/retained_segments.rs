use super::SegmentMaterializeContext;
use super::browser_fields::{apply_browser_frame, apply_browser_tag_sidebar};
use super::segment_keys::segment_key_changed;
use crate::app_core::actions::NativeDirtySegments;
use crate::app_core::ui_bridge::projection_cache::ProjectionSegment;
use crate::app_core::ui_projection;

type SegmentMaterializer = for<'ctx> fn(&mut SegmentMaterializeContext<'ctx>) -> bool;

#[derive(Clone, Copy)]
struct ProjectionSegmentHandler {
    segment: ProjectionSegment,
    dirty_bits: u16,
    materialize: SegmentMaterializer,
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
pub(super) fn retained_segment_handler_plan() -> Vec<(ProjectionSegment, u16)> {
    RETAINED_SEGMENT_HANDLERS
        .iter()
        .map(|handler| (handler.segment, handler.dirty_bits))
        .collect()
}

pub(super) fn record_full_cache_hit(
    cache: &mut crate::app_core::ui_bridge::projection_cache::UiProjectionCache,
) {
    for handler in RETAINED_SEGMENT_HANDLERS {
        cache.record_segment_lookup(handler.segment, true);
    }
}

pub(super) fn materialize_retained_segments(
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
