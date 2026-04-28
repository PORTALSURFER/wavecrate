use self::sempal_crate::app as native_model;
use super::StaticFrameCtx;
use super::*;
use crate as sempal_crate;

pub(super) fn render_map_panel(ctx: &StaticFrameCtx<'_>, primitives: &mut impl PrimitiveSink) {
    let canvas = compute_browser_map_canvas_rect(ctx.layout.browser_rows, ctx.sizing);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: canvas,
            color: blend_color(ctx.style.surface_base, ctx.style.bg_secondary, 0.24),
        }),
    );
    push_border(
        primitives,
        canvas,
        ctx.style.border_emphasis,
        ctx.sizing.border_width,
    );
    for point in ctx.model.map.points.iter() {
        let center = compute_browser_map_point_center(canvas, point.x_milli, point.y_milli);
        let color = map_point_color(ctx.style, ctx.model, point);
        let radius = if map_point_is_focused(ctx.model, point) {
            4.5
        } else if map_point_is_selected(ctx.model, point) {
            3.8
        } else {
            2.6
        };
        emit_primitive(
            primitives,
            Primitive::Circle(FillCircle {
                center,
                radius,
                color,
            }),
        );
    }
}

pub(super) fn render_map_header(ctx: &StaticFrameCtx<'_>, text_runs: &mut impl TextRunSink) {
    let mode_label = match ctx.model.map.render_mode {
        native_model::MapRenderModeModel::Heatmap => "heatmap",
        native_model::MapRenderModeModel::Points => "points",
    };
    let legend_text = if ctx.model.map.legend_label.is_empty() {
        format!(
            "{}: {mode_label}",
            ctx.model.browser_chrome.similarity_toggle_label
        )
    } else {
        ctx.model.map.legend_label.clone()
    };
    let header_left_text = format!(
        "{} | {}",
        ctx.model.browser_chrome.map_tab_label, legend_text
    );
    let selection_or_error = if let Some(error) = ctx.model.map.error.as_deref() {
        (error.to_string(), ctx.style.accent_warning)
    } else if !ctx.model.map.selection_label.is_empty() {
        (ctx.model.map.selection_label.clone(), ctx.style.text_muted)
    } else if !ctx.model.map.hover_label.is_empty() {
        (ctx.model.map.hover_label.clone(), ctx.style.text_muted)
    } else {
        (String::from("Selection: —"), ctx.style.text_muted)
    };
    let layout =
        compute_browser_map_header_text_layout(ctx.layout.browser_table_header, ctx.sizing);
    let left_max_width = layout.left_label.width().max(24.0);
    let right_max_width = layout.right_label.width().max(36.0);
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&header_left_text, left_max_width, ctx.sizing.font_meta),
            position: layout.left_label.min,
            font_size: ctx.sizing.font_meta,
            color: ctx.style.text_primary,
            max_width: Some(left_max_width),
            align: TextAlign::Left,
        },
    );
    emit_text(
        text_runs,
        TextRun {
            text: truncate_to_width(&selection_or_error.0, right_max_width, ctx.sizing.font_meta),
            position: layout.right_label.min,
            font_size: ctx.sizing.font_meta,
            color: selection_or_error.1,
            max_width: Some(right_max_width),
            align: TextAlign::Right,
        },
    );
}
