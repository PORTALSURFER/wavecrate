use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
    },
    widgets::TextWrap,
};

pub(super) fn background(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, rect: Rect) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color: button_fill(),
    }));
}

pub(super) fn hover(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    pressed: bool,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color: hover_fill(pressed),
    }));
}

pub(super) fn border(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, bounds: Rect) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: Rect::from_min_max(
            Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
            Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
        ),
        color: button_border(),
        width: 1.0,
    }));
}

pub(super) fn label(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    label: &str,
) {
    let text_rect = Rect::from_min_max(
        Point::new(bounds.min.x + 8.0, bounds.min.y),
        Point::new(bounds.max.x - 8.0, bounds.max.y),
    );
    let font_size = 11.0;
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from(label),
        rect: text_rect,
        font_size,
        baseline: Some(((text_rect.height() - font_size) * 0.5 + font_size * 0.78).round()),
        color: text_color(),
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
    }));
}

fn button_fill() -> Rgba8 {
    Rgba8 {
        r: 30,
        g: 31,
        b: 31,
        a: 235,
    }
}

fn button_border() -> Rgba8 {
    Rgba8 {
        r: 66,
        g: 67,
        b: 68,
        a: 175,
    }
}

pub(super) fn hover_fill(pressed: bool) -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 108,
        b: 88,
        a: if pressed { 170 } else { 155 },
    }
}

pub(super) fn text_color() -> Rgba8 {
    Rgba8 {
        r: 218,
        g: 219,
        b: 219,
        a: 238,
    }
}
