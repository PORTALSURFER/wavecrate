use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

pub(in crate::gui::native_shell::state) fn map_point_is_selected(
    model: &AppModel,
    point: &native_model::MapPointModel,
) -> bool {
    model.map.selected_item_id.as_deref() == Some(point.id.as_ref())
}

pub(in crate::gui::native_shell::state) fn map_point_is_focused(
    model: &AppModel,
    point: &native_model::MapPointModel,
) -> bool {
    model.map.focused_item_id.as_deref() == Some(point.id.as_ref())
}

pub(in crate::gui::native_shell::state) fn map_point_color(
    style: &StyleTokens,
    model: &AppModel,
    point: &native_model::MapPointModel,
) -> Rgba8 {
    if map_point_is_focused(model, point) {
        return style.accent_warning;
    }
    if map_point_is_selected(model, point) {
        return style.accent_mint;
    }
    match point.cluster_id.map(|id| id.rem_euclid(5)) {
        Some(0) => blend_color(style.accent_mint, style.bg_secondary, 0.42),
        Some(1) => blend_color(style.accent_copper, style.bg_secondary, 0.42),
        Some(2) => blend_color(style.accent_warning, style.bg_secondary, 0.42),
        Some(3) => blend_color(style.text_primary, style.bg_secondary, 0.35),
        Some(_) => blend_color(style.text_muted, style.bg_secondary, 0.35),
        None => blend_color(style.text_muted, style.bg_secondary, 0.5),
    }
}

pub(in crate::gui::native_shell::state) fn map_content_id_at_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<String> {
    if !model.map.active || model.map.points.is_empty() {
        return None;
    }
    let canvas =
        compute_browser_map_canvas_rect(layout.browser_rows, style_for_layout(layout).sizing);
    if !canvas.contains(point) {
        return None;
    }

    let mut best: Option<(f32, &str)> = None;
    for map_point in model.map.points.iter() {
        let center = compute_browser_map_point_center(canvas, map_point.x_milli, map_point.y_milli);
        let radius = if map_point_is_focused(model, map_point) {
            7.0
        } else if map_point_is_selected(model, map_point) {
            6.0
        } else {
            5.0
        };
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let distance_sq = (dx * dx) + (dy * dy);
        if distance_sq > (radius * radius) {
            continue;
        }
        match best {
            Some((best_distance_sq, _)) if distance_sq >= best_distance_sq => {}
            _ => best = Some((distance_sq, map_point.id.as_ref())),
        }
    }
    best.map(|(_, content_id)| content_id.to_string())
}
