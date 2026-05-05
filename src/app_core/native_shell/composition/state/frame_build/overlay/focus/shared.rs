use super::*;

pub(super) fn push_browser_row_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    if sides.top {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
                color,
            }),
        );
    }
    if sides.bottom {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
                color,
            }),
        );
    }
    if sides.left {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
                color,
            }),
        );
    }
    if sides.right {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
                color,
            }),
        );
    }
}

pub(super) fn render_section_focus_surface(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    style: &StyleTokens,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect,
            color: translucent_overlay_color(
                style.bg_tertiary,
                style.accent_warning,
                (style.state_focus_pulse_blend * 0.12).clamp(0.06, 0.16),
            ),
        }),
    );
    push_border(
        primitives,
        rect,
        blend_color(
            style.accent_warning,
            style.text_primary,
            style.state_focus_pulse_blend,
        ),
        style.sizing.focus_stroke_width,
    );
}

pub(super) fn union_rect(first: Rect, second: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(first.min.x.min(second.min.x), first.min.y.min(second.min.y)),
        Point::new(first.max.x.max(second.max.x), first.max.y.max(second.max.y)),
    )
}
