use super::super::super::projection_key_encoding::encode_browser_tab;
use super::super::MapProjectionCacheKey;
use crate::app_core::controller::AppController;

/// Build a map-panel projection key from the current controller snapshot.
pub(super) fn build_map_projection_key(controller: &AppController) -> MapProjectionCacheKey {
    MapProjectionCacheKey {
        map_open: controller.ui.map.open,
        map_zoom_bits: controller.ui.map.zoom.to_bits(),
        map_pan_x_bits: controller.ui.map.pan.x.to_bits(),
        map_pan_y_bits: controller.ui.map.pan.y.to_bits(),
        map_selection_revision: controller.ui.projection_revisions.map_selection,
        map_hover_revision: controller.ui.projection_revisions.map_hover,
        map_dataset_revision: controller.ui.projection_revisions.map_dataset,
        map_query_revision: controller.ui.projection_revisions.map_query,
        map_points_revision: controller.ui.map.cached_points_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}
