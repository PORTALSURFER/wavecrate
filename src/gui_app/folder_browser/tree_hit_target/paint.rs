use super::*;
use radiant::runtime::{PaintStrokeRect, PaintTextAlign, PaintTextRun};

impl FolderTreeHitTarget {
    pub(super) fn paint_background(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        let Some(color) = self.background_fill() else {
            return;
        };
        ui::push_fill_rect(primitives, self.row.common.id, bounds, color);
    }

    pub(super) fn paint_drop_target_outline(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if !self.drop_target {
            return;
        }
        let Some(rect) = ui::dense_row_inset_rect(bounds, 0.5) else {
            return;
        };
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.row.common.id,
            rect,
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
        let label_rect =
            ui::centered_text_line(bounds, font_size, ui::TextLineInsets::horizontal(4.0), 0.0);
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.row.common.id,
            text: self.label.clone(),
            rect: label_rect,
            font_size,
            baseline: ui::centered_text_baseline(label_rect, font_size),
            color: self.label_color(theme),
            align: PaintTextAlign::Left,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }

    fn background_fill(&self) -> Option<Rgba8> {
        let interaction_fill = Rgba8 {
            r: 255,
            g: 110,
            b: 85,
            a: if self.row.common.state.pressed {
                120
            } else {
                80
            },
        };
        ui::dense_row_fill_color(
            ui::DenseRowVisualState {
                selected: self.selected,
                hovered: self.row.common.state.hovered,
                pressed: self.row.common.state.pressed,
                active_target: self.drop_target,
                candidate: self.drop_candidate,
            },
            ui::DenseRowPalette::new()
                .selected(Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 105,
                })
                .hovered(interaction_fill)
                .pressed(interaction_fill)
                .active_target(Rgba8 {
                    r: 255,
                    g: 130,
                    b: 78,
                    a: 150,
                })
                .candidate_hovered(Rgba8 {
                    r: 255,
                    g: 122,
                    b: 74,
                    a: 110,
                }),
        )
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
