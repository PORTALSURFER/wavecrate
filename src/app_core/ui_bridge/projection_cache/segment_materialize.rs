#[cfg(test)]
use super::ProjectionSegment;
use super::{DerivedProjectionState, UiProjectionCache, trace_projection_cache_lookup};
use crate::app_core::actions::{NativeAppModel, NativeDirtySegments};
use crate::app_core::controller::AppController;
use std::sync::Arc;

mod browser_fields;
mod non_segment;
mod retained_segments;
mod segment_keys;

struct SegmentMaterializeContext<'a> {
    cache: &'a mut UiProjectionCache,
    model: &'a mut NativeAppModel,
    controller: &'a mut AppController,
    derived: &'a DerivedProjectionState,
    has_retained_model: bool,
}

#[cfg(test)]
pub(crate) fn retained_segment_handler_plan() -> Vec<(ProjectionSegment, u16)> {
    retained_segments::retained_segment_handler_plan()
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
        retained_segments::record_full_cache_hit(cache);
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
        retained_segments::materialize_retained_segments(&mut segment_context, &mut dirty_segments);
    }

    if non_segment::update_static_key(cache, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::GLOBAL_STATIC);
        non_segment::refresh_static_fields(model, controller);
        non_segment::refresh_audio_chip_fields(model, &controller.ui);
    }

    if non_segment::update_overlay_key(cache, derived, has_retained_model) {
        dirty_segments.insert(NativeDirtySegments::STATE_OVERLAY);
        non_segment::refresh_overlay_fields(model, controller);
    }

    non_segment::refresh_always_fields(model, derived.selected_column);
    cache.app_key = Some(derived.app_key.clone());
    cache.app_model = Some(Arc::clone(&snapshot));
    (snapshot, dirty_segments)
}
