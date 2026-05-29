use super::*;
use radiant::runtime::{PaintFillRect, PaintStrokeRect, PaintTextAlign, PaintTextRun};

impl FolderTreeHitTarget {
    pub(super) fn paint_background(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        let Some(color) = self.background_fill() else {
            return;
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: bounds,
            color,
        }));
    }

    pub(super) fn paint_drop_target_outline(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if !self.drop_target {
            return;
        }
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.row.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
            ),
            color: Rgba8 {
                r: 255,
                g: 180,
                b: 130,
                a: 210,
            },
            width: 1.0,
        }));
    }

    pub(super) fn paint_label(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        let font_size = label_font_size(bounds);
        let label_rect = Rect::from_min_max(
            Point::new(bounds.min.x + 4.0, bounds.min.y),
            Point::new(bounds.max.x - 4.0, bounds.max.y),
        );
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.row.common.id,
            text: self.label.clone(),
            rect: label_rect,
            font_size,
            baseline: Some((label_rect.height() * 0.5 + font_size * 0.35).max(0.0)),
            color: self.label_color(theme),
            align: PaintTextAlign::Left,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }

    fn background_fill(&self) -> Option<Rgba8> {
        if self.drop_target {
            Some(Rgba8 {
                r: 255,
                g: 130,
                b: 78,
                a: 150,
            })
        } else if self.row.common.state.hovered && self.drop_candidate {
            Some(Rgba8 {
                r: 255,
                g: 122,
                b: 74,
                a: 110,
            })
        } else if self.row.common.state.pressed || self.row.common.state.hovered {
            Some(Rgba8 {
                r: 255,
                g: 110,
                b: 85,
                a: if self.row.common.state.pressed {
                    120
                } else {
                    80
                },
            })
        } else if self.selected {
            Some(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 105,
            })
        } else {
            None
        }
    }

    fn label_color(&self, theme: &ThemeTokens) -> Rgba8 {
        if self.label_is_highlighted() {
            Rgba8 {
                r: 255,
                g: 238,
                b: 224,
                a: 255,
            }
        } else {
            theme.text_primary
        }
    }

    fn label_is_highlighted(&self) -> bool {
        self.drop_target || (self.row.common.state.hovered && self.drop_candidate) || self.selected
    }
}

fn label_font_size(bounds: Rect) -> f32 {
    if bounds.height() >= 38.0 {
        18.0
    } else if bounds.height() >= 28.0 {
        14.0
    } else {
        13.0
    }
}
