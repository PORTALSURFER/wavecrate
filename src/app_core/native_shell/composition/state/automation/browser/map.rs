use super::*;

pub(super) fn map_canvas_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let canvas = compute_browser_map_canvas_rect(layout.browser_rows, style.sizing);
    let mut map_node = simple_node(
        "browser.map_canvas",
        AutomationRole::MapCanvas,
        Some(model.browser_chrome.map_tab_label.clone()),
        canvas,
        Some(model.map.summary.clone()),
        true,
        true,
        vec![String::from("focus_spatial_content_item")],
    );
    map_node.children = model
        .map
        .points
        .iter()
        .map(|point| AutomationNodeSnapshot {
            id: node_id(format!("browser.map.point.{}", point.id)),
            role: AutomationRole::MapPoint,
            label: Some(String::from(point.id.as_ref())),
            bounds: bounds(circle_rect(
                compute_browser_map_point_center(canvas, point.x_milli, point.y_milli),
                10.0,
            )),
            value: None,
            enabled: true,
            selected: model.map.selected_item_id.as_deref() == Some(point.id.as_ref()),
            available_actions: vec![String::from("focus_spatial_content_item")],
            metadata: metadata(&[
                ("x_milli", &point.x_milli.to_string()),
                ("y_milli", &point.y_milli.to_string()),
            ]),
            children: Vec::new(),
        })
        .collect();
    map_node
}
