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

const RATING_KEEP_COLOR: Rgba8 = Rgba8 {
    r: 122,
    g: 226,
    b: 96,
    a: 235,
};
const RATING_TRASH_COLOR: Rgba8 = Rgba8 {
    r: 238,
    g: 77,
    b: 67,
    a: 235,
};

#[derive(Clone, Debug)]
pub(super) struct RatingIndicator {
    rating: Rating,
    locked: bool,
}

impl RatingIndicator {
    pub(super) fn new(rating: Rating, locked: bool) -> Self {
        Self { rating, locked }
    }

    pub(super) fn count(&self) -> usize {
        self.rating.val().unsigned_abs().min(3) as usize
    }

    pub(super) fn color(&self) -> Option<Rgba8> {
        if self.rating.is_keep() {
            Some(RATING_KEEP_COLOR)
        } else if self.rating.is_trash() {
            Some(RATING_TRASH_COLOR)
        } else {
            None
        }
    }

    pub(super) fn shows_keep_badge(&self) -> bool {
        self.locked && self.rating == Rating::KEEP_3
    }
}

#[derive(Clone, Debug)]
pub(super) struct KeepBadge {
    common: WidgetCommon,
}

impl KeepBadge {
    pub(super) fn new() -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common }
    }
}

impl Widget for KeepBadge {
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
        paint_keep_badge(primitives, self.common.id, bounds);
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
