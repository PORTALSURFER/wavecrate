//! Static-scene rebuild policy helpers for native-vello frame updates.

use super::*;

/// Resolve static-scene rebuild behavior for one frame update.
///
/// `static_rebuild_requested` represents explicit runtime invalidations
/// (for example layout or full-static scopes). When no explicit static rebuild
/// is requested, bridge dirty segments decide whether static browser must
/// rebuild during model refreshes.
pub(super) fn resolve_static_rebuild(
    model_refresh_requested: bool,
    static_rebuild_requested: bool,
    bridge_dirty_segments: DirtySegments,
) -> bool {
    if !model_refresh_requested {
        return static_rebuild_requested;
    }
    if bridge_dirty_segments.requires_static_rebuild() {
        return true;
    }
    static_rebuild_requested
}

/// Return whether bridge dirty segments forced a static rebuild for this refresh.
pub(super) fn static_rebuild_from_dirty_mask(
    model_refresh_requested: bool,
    static_rebuild_requested: bool,
    bridge_dirty_segments: DirtySegments,
) -> bool {
    model_refresh_requested
        && !static_rebuild_requested
        && bridge_dirty_segments.requires_static_rebuild()
}
