use crate::app_core::actions::NativeUiAction;

mod source;
mod targeted;

pub(super) fn prefers_targeted_invalidation(action: &NativeUiAction) -> bool {
    targeted::prefers_targeted_invalidation(action)
}

pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(super::InvalidationSource, super::InvalidationReason)> {
    source::classify_dirty_source(action)
}
