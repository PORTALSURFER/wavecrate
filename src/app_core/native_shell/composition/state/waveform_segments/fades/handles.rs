use super::super::*;
use super::model::{EditFadeOverlayGeometry, EditFadePositions};
use super::{EDIT_FADE_HANDLE_TAB_SIZE, EDIT_FADE_HANDLE_WIDTH};

pub(super) fn emit_fade_handles(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    positions: EditFadePositions,
    accent_blue: Rgba8,
) {
    emit_edit_fade_handle(
        primitives,
        style,
        geometry,
        positions.fade_in_x,
        accent_blue,
    );
    if positions.has_fade_in {
        emit_edit_fade_bottom_handle(
            primitives,
            style,
            geometry,
            positions.fade_in_mute_x,
            accent_blue,
        );
    }

    emit_edit_fade_handle(
        primitives,
        style,
        geometry,
        positions.fade_out_x,
        accent_blue,
    );
    if positions.has_fade_out {
        emit_edit_fade_bottom_handle(
            primitives,
            style,
            geometry,
            positions.fade_out_mute_x,
            accent_blue,
        );
    }
}

fn emit_edit_fade_handle(
    primitives: &mut impl PrimitiveSink,
    style: &StyleTokens,
    geometry: EditFadeOverlayGeometry,
    x: f32,
    accent_blue: Rgba8,
) {
    let tab = edit_fade_handle_tab_rect(geometry, x, style.sizing.border_width);
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
    geometry: EditFadeOverlayGeometry,
    x: f32,
    accent_blue: Rgba8,
) {
    let handle = edit_fade_bottom_handle_rect(geometry, x, style.sizing.border_width);
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: handle,
            color: translucent_overlay_color(style.bg_secondary, accent_blue, 0.38),
        }),
    );

    let bottom_tab = edit_fade_handle_bottom_tab_rect(geometry, x, style.sizing.border_width);
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

fn edit_fade_bottom_handle_rect(
    geometry: EditFadeOverlayGeometry,
    x: f32,
    border_width: f32,
) -> Rect {
    let width = EDIT_FADE_HANDLE_WIDTH.max(border_width).max(1.0);
    let half = width * 0.5;
    let left = (x - half).clamp(
        geometry.waveform_plot.min.x,
        geometry.waveform_plot.max.x - 1.0,
    );
    let right = (left + width)
        .min(geometry.waveform_plot.max.x)
        .max(left + 1.0);
    Rect::from_min_max(
        Point::new(left, geometry.edit_selection_rect.min.y),
        Point::new(right, geometry.edit_selection_rect.max.y),
    )
}

fn edit_fade_handle_tab_rect(geometry: EditFadeOverlayGeometry, x: f32, border_width: f32) -> Rect {
    let size = handle_tab_size(geometry, border_width);
    let left = handle_tab_left(geometry, x, size);
    let bottom = (geometry.edit_selection_rect.min.y + size)
        .min(geometry.edit_selection_rect.max.y)
        .max(geometry.edit_selection_rect.min.y + 1.0);
    Rect::from_min_max(
        Point::new(left, geometry.edit_selection_rect.min.y),
        Point::new((left + size).min(geometry.waveform_plot.max.x), bottom),
    )
}

fn edit_fade_handle_bottom_tab_rect(
    geometry: EditFadeOverlayGeometry,
    x: f32,
    border_width: f32,
) -> Rect {
    let size = handle_tab_size(geometry, border_width);
    let left = handle_tab_left(geometry, x, size);
    let top = (geometry.edit_selection_rect.max.y - size)
        .max(geometry.edit_selection_rect.min.y)
        .min(geometry.edit_selection_rect.max.y - 1.0);
    Rect::from_min_max(
        Point::new(left, top),
        Point::new(
            (left + size).min(geometry.waveform_plot.max.x),
            geometry.edit_selection_rect.max.y,
        ),
    )
}

fn handle_tab_size(geometry: EditFadeOverlayGeometry, border_width: f32) -> f32 {
    EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .max(border_width + 2.0)
        .min(geometry.edit_selection_rect.height().max(1.0))
        .min(geometry.waveform_plot.width().max(1.0))
}

fn handle_tab_left(geometry: EditFadeOverlayGeometry, x: f32, size: f32) -> f32 {
    (x - (size * 0.5)).clamp(
        geometry.waveform_plot.min.x,
        geometry.waveform_plot.max.x - size.max(1.0),
    )
}
