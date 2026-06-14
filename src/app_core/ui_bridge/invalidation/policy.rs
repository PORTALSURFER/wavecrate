use super::{InvalidationReason, InvalidationSource};
use crate::app_core::actions::NativeUiAction;
#[cfg(test)]
use crate::app_core::actions::{GuiActionKind, representative_action_for_kind};

mod browser;
mod status;
mod transport;
mod waveform;

/// Return whether an action requires unconditional projection-cache invalidation.
pub(in crate::app_core::ui_bridge) fn action_requires_projection_cache_invalidation(
    action: &NativeUiAction,
) -> bool {
    !waveform::can_skip_projection_cache_invalidation(action)
        && !transport::can_skip_projection_cache_invalidation(action)
}

/// Conservative source-node set used for broad invalidation actions.
pub(in crate::app_core::ui_bridge) const BROAD_DIRTY_SOURCES: [InvalidationSource; 4] = [
    InvalidationSource::Browser,
    InvalidationSource::Map,
    InvalidationSource::Transport,
    InvalidationSource::Status,
];

/// Return whether an action should stay on targeted dirty-source invalidation.
///
/// High-frequency browser navigation/search actions are intentionally excluded
/// from broad invalidation because broad invalidation fans out to unrelated
/// map/transport/status sources and increases projection-key churn.
pub(in crate::app_core::ui_bridge) fn action_prefers_targeted_invalidation(
    action: &NativeUiAction,
) -> bool {
    browser::prefers_targeted_invalidation(action)
}

/// Resolve the primary dirty source node and reason for one UI action.
pub(in crate::app_core::ui_bridge) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    waveform::classify_dirty_source(action)
        .or_else(|| browser::classify_dirty_source(action))
        .or_else(|| transport::classify_dirty_source(action))
        .or_else(|| status::classify_dirty_source(action))
}

/// Return whether dirty waveform render inputs require a full image refresh.
pub(in crate::app_core::ui_bridge) fn waveform_render_inputs_require_refresh(
    reason: Option<InvalidationReason>,
) -> bool {
    waveform::render_inputs_require_refresh(reason)
}

/// Resolve whether one catalog action kind prefers targeted invalidation.
#[cfg(test)]
pub(crate) fn catalog_prefers_targeted_invalidation(kind: GuiActionKind) -> bool {
    action_prefers_targeted_invalidation(&representative_action_for_kind(kind))
}

/// Resolve the dirty source metadata for one catalog action kind.
#[cfg(test)]
pub(crate) fn catalog_dirty_source(
    kind: GuiActionKind,
) -> Option<(InvalidationSource, InvalidationReason)> {
    classify_dirty_source(&representative_action_for_kind(kind))
}
