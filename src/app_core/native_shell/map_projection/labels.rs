use super::*;
use std::{path::Path, sync::Arc};

/// Resolve the projected native-shell render mode and its label.
pub(super) fn map_render_mode_parts(
    controller: &AppController,
) -> (MapRenderModeModel, &'static str) {
    let render_mode = match MapRenderMode::from(controller.ui.map.last_render_mode) {
        MapRenderMode::Heatmap => MapRenderModeModel::Heatmap,
        MapRenderMode::Points => MapRenderModeModel::Points,
    };
    let render_mode_label = match render_mode {
        MapRenderModeModel::Heatmap => "heatmap",
        MapRenderModeModel::Points => "points",
    };
    (render_mode, render_mode_label)
}

/// Build the inactive map panel model.
pub(super) fn build_hidden_map_panel_model(
    controller: &AppController,
    render_mode: MapRenderModeModel,
    render_mode_label: &str,
) -> MapPanelModel {
    MapPanelModel {
        active: false,
        summary: String::from("Map hidden"),
        legend_label: format!("Render: {render_mode_label}"),
        selection_label: String::from("Selection: —"),
        hover_label: String::from("Hover: —"),
        cluster_label: String::from("Clusters: —"),
        viewport_label: viewport_label(controller),
        error: None,
        render_mode,
        selected_sample_id: None,
        focused_sample_id: None,
        points: Arc::default(),
    }
}

/// Build the active "no data yet" map panel model.
pub(super) fn build_map_unavailable_model(
    controller: &AppController,
    render_mode: MapRenderModeModel,
    render_mode_label: &str,
) -> MapPanelModel {
    MapPanelModel {
        active: true,
        summary: String::from("No map data (run similarity prep)"),
        legend_label: format!("Render: {render_mode_label}"),
        selection_label: String::from("Selection: —"),
        hover_label: String::from("Hover: —"),
        cluster_label: String::from("Clusters: —"),
        viewport_label: viewport_label(controller),
        error: None,
        render_mode,
        selected_sample_id: None,
        focused_sample_id: None,
        points: Arc::default(),
    }
}

/// Build an error map panel model using caller-supplied state labels.
pub(super) fn build_map_query_error_model(
    controller: &AppController,
    render_mode: MapRenderModeModel,
    render_mode_label: &str,
    summary: String,
    selection_label: String,
    hover_label: String,
    cluster_label: String,
    err: String,
) -> MapPanelModel {
    MapPanelModel {
        active: true,
        summary,
        legend_label: format!("Render: {render_mode_label}"),
        selection_label,
        hover_label,
        cluster_label,
        viewport_label: viewport_label(controller),
        error: Some(err),
        render_mode,
        selected_sample_id: None,
        focused_sample_id: None,
        points: Arc::default(),
    }
}

/// Build the active map panel model from retained projection caches.
pub(super) fn build_active_map_panel_model(
    controller: &AppController,
    render_mode: MapRenderModeModel,
    render_mode_label: &str,
    selected_sample_id: Option<String>,
    focused_sample_id: Option<String>,
) -> MapPanelModel {
    let cluster_count = controller.projected_map_cluster_count;
    let points = Arc::clone(&controller.projected_map_points);
    MapPanelModel {
        active: true,
        summary: format!("{} points", points.len()),
        legend_label: format!("Render: {render_mode_label}"),
        selection_label: selection_label(controller, focused_sample_id.as_deref()),
        hover_label: hover_label(controller),
        cluster_label: cluster_label(cluster_count),
        viewport_label: viewport_label(controller),
        error: None,
        render_mode,
        selected_sample_id,
        focused_sample_id,
        points,
    }
}

fn selection_label(controller: &AppController, focused_sample_id: Option<&str>) -> String {
    controller
        .ui
        .map
        .selected_sample_id
        .as_deref()
        .map(short_sample_label)
        .map(|label| format!("Selection: {label}"))
        .or_else(|| {
            focused_sample_id
                .map(short_sample_label)
                .map(|label| format!("Focus: {label}"))
        })
        .unwrap_or_else(|| String::from("Selection: —"))
}

fn hover_label(controller: &AppController) -> String {
    controller
        .ui
        .map
        .hovered_sample_id
        .as_deref()
        .or(controller.ui.map.paint_hover_active_id.as_deref())
        .map(short_sample_label)
        .map(|label| format!("Hover: {label}"))
        .unwrap_or_else(|| String::from("Hover: —"))
}

fn cluster_label(cluster_count: usize) -> String {
    if cluster_count == 0 {
        String::from("Clusters: —")
    } else {
        format!("Clusters: {cluster_count}")
    }
}

fn viewport_label(controller: &AppController) -> String {
    format!(
        "zoom {:.2}x | pan ({:.0}, {:.0})",
        controller.ui.map.zoom, controller.ui.map.pan.x, controller.ui.map.pan.y
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_sample_label_prefers_filename_and_truncates_long_values() {
        let label = short_sample_label(
            "source::folder/abcdefghijklmnopqrstuvwxyz0123456789_kick.wav",
        );

        assert!(label.ends_with("..."));
        assert!(label.len() <= 32);
    }
}
