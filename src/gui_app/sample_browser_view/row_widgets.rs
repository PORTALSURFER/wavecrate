use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, TextWrap, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use wavecrate::sample_sources::Rating;
#[derive(Clone, Debug)]
pub(super) struct RatingSquares {
    common: WidgetCommon,
    rating: Rating,
    locked: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CollectionBlock {
    common: WidgetCommon,
    color: Option<Rgba8>,
}

impl CollectionBlock {
    pub(super) fn new(color: Option<Rgba8>) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, color }
    }
}

impl Widget for CollectionBlock {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        let Some(color) = self.color else {
            return;
        };
        let size = 10.0_f32.min(bounds.height().max(0.0));
        let x = bounds.max.x - size - 4.0;
        let y = bounds.min.y + (bounds.height() - size) * 0.5;
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(Point::new(x, y), Point::new(x + size, y + size)),
            color,
        }));
    }
}

impl RatingSquares {
    pub(super) fn new(rating: Rating, locked: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            rating,
            locked,
        }
    }

    pub(super) fn count(&self) -> usize {
        self.rating.val().unsigned_abs().min(3) as usize
    }

    fn color(&self) -> Option<Rgba8> {
        if self.rating.is_keep() {
            Some(Rgba8 {
                r: 122,
                g: 226,
                b: 96,
                a: 235,
            })
        } else if self.rating.is_trash() {
            Some(Rgba8 {
                r: 238,
                g: 77,
                b: 67,
                a: 235,
            })
        } else {
            None
        }
    }
}

impl Widget for RatingSquares {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if self.locked && self.rating == Rating::KEEP_3 {
            paint_keep_badge(primitives, self.common.id, bounds);
            return;
        }
        let Some(color) = self.color() else {
            return;
        };
        let count = self.count();
        if count == 0 {
            return;
        }

        let size = 5.0_f32.min(bounds.height().max(0.0));
        let gap = 4.0;
        let total_width = count as f32 * size + count.saturating_sub(1) as f32 * gap;
        let start_x = bounds.max.x - total_width - 4.0;
        let y = bounds.min.y + (bounds.height() - size) * 0.5;
        for index in 0..count {
            let x = start_x + index as f32 * (size + gap);
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(Point::new(x, y), Point::new(x + size, y + size)),
                color,
            }));
        }
    }
}

fn paint_keep_badge(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, bounds: Rect) {
    let badge_width = 38.0_f32.min(bounds.width().max(0.0));
    let badge_height = 14.0_f32.min(bounds.height().max(0.0));
    if badge_width <= 0.0 || badge_height <= 0.0 {
        return;
    }
    let x = bounds.max.x - badge_width - 2.0;
    let y = bounds.min.y + (bounds.height() - badge_height) * 0.5;
    let rect = Rect::from_min_max(
        Point::new(x, y),
        Point::new(x + badge_width, y + badge_height),
    );
    let gold = Rgba8 {
        r: 221,
        g: 177,
        b: 54,
        a: 245,
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color: Rgba8 {
            r: 54,
            g: 43,
            b: 14,
            a: 170,
        },
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color: gold,
        width: 1.0,
    }));
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from("KEEP"),
        rect,
        font_size: 9.0,
        baseline: Some(((rect.height() - 9.0) * 0.5 + 9.0 * 0.78).round()),
        color: gold,
        align: PaintTextAlign::Center,
        wrap: TextWrap::None,
    }));
}
