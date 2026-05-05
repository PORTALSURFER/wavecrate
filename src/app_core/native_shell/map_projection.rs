//! Map-panel projection and retained map-point cache helpers.

use super::*;

mod cache;
mod labels;
mod points;

use self::cache::{map_projection_cache_key, refresh_map_points_query_cache, resolve_map_bounds};
use self::labels::{
    build_active_map_panel_model, build_hidden_map_panel_model, build_map_query_error_model,
    build_map_unavailable_model, map_render_mode_parts,
};
use self::points::refresh_projected_map_points_cache;

/// Project map panel state, query results, and retained point payloads.
///
/// The projection reuses cached bounds/points when source/version/query keys
/// match and only falls back to DB queries when cache inputs become stale.
pub(crate) fn project_map_model(controller: &mut AppController) -> MapPanelModel {
    let active = matches!(
        SampleBrowserTab::from(controller.ui.browser.active_tab),
        SampleBrowserTab::Map
    );
    let (render_mode, render_mode_label) = map_render_mode_parts(controller);
    if !active {
        return build_hidden_map_panel_model(controller, render_mode, render_mode_label);
    }

    let source_id = controller.current_source().map(|source| source.id);
    let source_id_key = source_id.as_ref().map(|id| id.as_str().to_string());
    let umap_version = controller.ui.map.umap_version.clone();
    let bounds = match resolve_map_bounds(
        controller,
        source_id.as_ref(),
        &source_id_key,
        umap_version.as_str(),
    ) {
        Ok(Some(bounds)) => bounds,
        Ok(None) => return build_map_unavailable_model(controller, render_mode, render_mode_label),
        Err(err) => {
            return build_map_query_error_model(
                controller,
                render_mode,
                render_mode_label,
                String::from("Map unavailable"),
                String::from("Selection: unavailable"),
                String::from("Hover: unavailable"),
                String::from("Clusters: unavailable"),
                err,
            );
        }
    };

    let query_bounds = match refresh_map_points_query_cache(
        controller,
        source_id.as_ref(),
        &source_id_key,
        umap_version.as_str(),
        bounds,
    ) {
        Ok(query_bounds) => query_bounds,
        Err(err) => {
            return build_map_query_error_model(
                controller,
                render_mode,
                render_mode_label,
                String::from("Map query failed"),
                String::from("Selection: unavailable"),
                String::from("Hover: unavailable"),
                String::from("Clusters: unavailable"),
                err,
            );
        }
    };

    let focused_sample_id = controller.selected_sample_id();
    let selected_sample_id = controller.ui.map.selected_sample_id.clone();
    let projection_key = map_projection_cache_key(
        source_id_key.as_deref(),
        umap_version.as_str(),
        controller.ui.map.cached_points_revision,
        query_bounds,
    );
    refresh_projected_map_points_cache(controller, projection_key, bounds);

    build_active_map_panel_model(
        controller,
        render_mode,
        render_mode_label,
        selected_sample_id,
        focused_sample_id,
    )
}
