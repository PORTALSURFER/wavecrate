use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

/// Width in logical pixels for edit-fade drag handles.
const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;
/// Width/height in logical pixels for square edit-fade grab tabs.
const EDIT_FADE_HANDLE_TAB_SIZE: f32 = 10.0;

/// Emit edit-fade shading and draggable handle markers for the active edit selection.
pub(super) fn emit_edit_fade_overlays(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    edit_selection: native_model::NormalizedRangeModel,
    fade_in_end_milli: Option<u16>,
    fade_in_end_micros: Option<u32>,
    fade_in_mute_start_milli: Option<u16>,
    fade_in_mute_start_micros: Option<u32>,
    fade_in_curve_milli: Option<u16>,
    fade_out_start_milli: Option<u16>,
    fade_out_start_micros: Option<u32>,
    fade_out_mute_end_milli: Option<u16>,
    fade_out_mute_end_micros: Option<u32>,
    fade_out_curve_milli: Option<u16>,
    view_start_micros: u32,
    view_end_micros: u32,
    accent_blue: Rgba8,
) {
    let selection_start = edit_selection.start_micros.min(edit_selection.end_micros);
    let selection_end = edit_selection.start_micros.max(edit_selection.end_micros);
    if selection_end <= selection_start {
        return;
    }
    let fade_in_end = fade_in_end_micros
        .or_else(|| fade_in_end_milli.map(|value| u32::from(value) * 1000))
        .unwrap_or(selection_start)
        .clamp(selection_start, selection_end);
    let fade_out_start = fade_out_start_micros
        .or_else(|| fade_out_start_milli.map(|value| u32::from(value) * 1000))
        .unwrap_or(selection_end)
        .clamp(selection_start, selection_end);

    let view_width = (view_end_micros.saturating_sub(view_start_micros)).max(1) as f32;
    if waveform_plot.width() <= 0.0 {
        return;
    }

    let x_for_micros = |micros: u32| {
        let in_view = (((micros as f32) - (view_start_micros as f32)) / view_width).clamp(0.0, 1.0);
        waveform_plot.min.x + (waveform_plot.width() * in_view)
    };
    let fade_in_mute_start = fade_in_mute_start_micros
        .or_else(|| fade_in_mute_start_milli.map(|value| u32::from(value) * 1000))
        .unwrap_or(selection_start)
        .min(selection_start);
    let fade_in_x = x_for_micros(fade_in_end).clamp(waveform_plot.min.x, waveform_plot.max.x);
    let has_fade_in = fade_in_end > selection_start;
    let fade_in_mute_x =
        x_for_micros(fade_in_mute_start).clamp(waveform_plot.min.x, waveform_plot.max.x);
    let fade_out_x = x_for_micros(fade_out_start).clamp(waveform_plot.min.x, waveform_plot.max.x);
    let fade_out_mute_end = fade_out_mute_end_micros
        .or_else(|| fade_out_mute_end_milli.map(|value| u32::from(value) * 1000))
        .unwrap_or(selection_end)
        .max(selection_end);
    let has_fade_out = fade_out_start < selection_end;
    let fade_out_mute_x =
        x_for_micros(fade_out_mute_end).clamp(waveform_plot.min.x, waveform_plot.max.x);

    if fade_in_x > edit_selection_rect.min.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(edit_selection_rect.min.x, edit_selection_rect.min.y),
                    Point::new(fade_in_x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.22),
            }),
        );
        emit_edit_fade_curve_trace(
            primitives,
            style,
            waveform_plot,
            edit_selection_rect,
            fade_in_mute_x,
            fade_in_x,
            fade_in_curve_milli.unwrap_or(500),
            true,
            accent_blue,
        );
    }
    if fade_in_mute_x < edit_selection_rect.min.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(fade_in_mute_x, edit_selection_rect.min.y),
                    Point::new(edit_selection_rect.min.x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.16),
            }),
        );
    }
    if fade_out_x < edit_selection_rect.max.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(fade_out_x, edit_selection_rect.min.y),
                    Point::new(edit_selection_rect.max.x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.22),
            }),
        );
        emit_edit_fade_curve_trace(
            primitives,
            style,
            waveform_plot,
            edit_selection_rect,
            fade_out_x,
            fade_out_mute_x,
            fade_out_curve_milli.unwrap_or(500),
            false,
            accent_blue,
        );
    }
    if fade_out_mute_x > edit_selection_rect.max.x {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(
                    Point::new(edit_selection_rect.max.x, edit_selection_rect.min.y),
                    Point::new(fade_out_mute_x, edit_selection_rect.max.y),
                ),
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.16),
            }),
        );
    }

    emit_edit_fade_handle(
        primitives,
        style,
        waveform_plot,
        edit_selection_rect,
        fade_in_x,
        accent_blue,
    );
    if has_fade_in {
        emit_edit_fade_bottom_handle(
            primitives,
            style,
            waveform_plot,
            edit_selection_rect,
            fade_in_mute_x,
            accent_blue,
        );
    }
    emit_edit_fade_handle(
        primitives,
        style,
        waveform_plot,
        edit_selection_rect,
        fade_out_x,
        accent_blue,
    );
    if has_fade_out {
        emit_edit_fade_bottom_handle(
            primitives,
            style,
            waveform_plot,
            edit_selection_rect,
            fade_out_mute_x,
            accent_blue,
        );
    }
}

fn emit_edit_fade_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    x: f32,
    accent_blue: Rgba8,
) {
    let tab = edit_fade_handle_tab_rect(
        waveform_plot,
        edit_selection_rect,
        x,
        style.sizing.border_width,
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tab,
            color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.78),
        }),
    );
    push_border(
        primitives,
        tab,
        blend_color(accent_blue, style.text_primary, 0.5),
        style.sizing.border_width,
    );
}

fn emit_edit_fade_bottom_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    x: f32,
    accent_blue: Rgba8,
) {
    let width = EDIT_FADE_HANDLE_WIDTH
        .max(style.sizing.border_width)
        .max(1.0);
    let half = width * 0.5;
    let left = (x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - 1.0);
    let right = (left + width).min(waveform_plot.max.x).max(left + 1.0);
    let handle = Rect::from_min_max(
        Point::new(left, edit_selection_rect.min.y),
        Point::new(right, edit_selection_rect.max.y),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.bg_secondary, accent_blue, 0.38),
        }),
    );
    let bottom_tab = edit_fade_handle_bottom_tab_rect(
        waveform_plot,
        edit_selection_rect,
        x,
        style.sizing.border_width,
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: bottom_tab,
            color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.78),
        }),
    );
    push_border(
        primitives,
        bottom_tab,
        blend_color(accent_blue, style.text_primary, 0.5),
        style.sizing.border_width,
    );
}

fn edit_fade_handle_tab_rect(
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    x: f32,
    border_width: f32,
) -> Rect {
    let size = EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .max(border_width + 2.0)
        .min(edit_selection_rect.height().max(1.0))
        .min(waveform_plot.width().max(1.0));
    let half = size * 0.5;
    let left = (x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - size.max(1.0));
    let right = (left + size).min(waveform_plot.max.x).max(left + 1.0);
    let bottom = (edit_selection_rect.min.y + size)
        .min(edit_selection_rect.max.y)
        .max(edit_selection_rect.min.y + 1.0);
    Rect::from_min_max(
        Point::new(left, edit_selection_rect.min.y),
        Point::new(right, bottom),
    )
}

fn edit_fade_handle_bottom_tab_rect(
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    x: f32,
    border_width: f32,
) -> Rect {
    let size = EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .max(border_width + 2.0)
        .min(edit_selection_rect.height().max(1.0));
    let half = size * 0.5;
    let left = (x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - size.max(1.0));
    let right = (left + size).min(waveform_plot.max.x).max(left + 1.0);
    let top = (edit_selection_rect.max.y - size)
        .max(edit_selection_rect.min.y)
        .min(edit_selection_rect.max.y - 1.0);
    Rect::from_min_max(
        Point::new(left, top),
        Point::new(right, edit_selection_rect.max.y),
    )
}

fn emit_edit_fade_curve_trace(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    waveform_plot: Rect,
    edit_selection_rect: Rect,
    start_x: f32,
    end_x: f32,
    curve_milli: u16,
    fade_in: bool,
    accent_blue: Rgba8,
) {
    let width = (end_x - start_x).abs();
    let height = edit_selection_rect.height();
    if width <= 1.0 || height <= 1.0 {
        return;
    }
    let curve = (f32::from(curve_milli.min(1000)) / 1000.0).clamp(0.0, 1.0);
    let steps = ((width / 6.0).round() as usize).clamp(6, 28);
    let marker_size = style.sizing.border_width.max(1.0) + 1.0;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let eased = fade_curve_sample(t, curve);
        let x = start_x + ((end_x - start_x) * t);
        let y = if fade_in {
            edit_selection_rect.max.y - (height * eased)
        } else {
            edit_selection_rect.min.y + (height * eased)
        };
        let rect = Rect::from_min_max(
            Point::new(
                (x - (marker_size * 0.5)).clamp(waveform_plot.min.x, waveform_plot.max.x),
                (y - (marker_size * 0.5))
                    .clamp(edit_selection_rect.min.y, edit_selection_rect.max.y),
            ),
            Point::new(
                (x + (marker_size * 0.5)).clamp(waveform_plot.min.x, waveform_plot.max.x),
                (y + (marker_size * 0.5))
                    .clamp(edit_selection_rect.min.y, edit_selection_rect.max.y),
            ),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: translucent_overlay_color(style.surface_overlay, accent_blue, 0.88),
            }),
        );
    }
}

fn fade_curve_sample(t: f32, curve: f32) -> f32 {
    if curve <= 0.0 {
        return t.clamp(0.0, 1.0);
    }
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let smootherstep = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    t * (1.0 - curve) + smootherstep * curve
}
