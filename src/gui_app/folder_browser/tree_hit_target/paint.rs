use super::*;

impl FolderTreeHitTarget {
    pub(super) fn paint_background(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        self.row.push_dense_fill(
            primitives,
            bounds,
            self.background_state_parts(),
            self.background_palette(),
        );
    }

    pub(super) fn paint_drop_target_outline(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if !self.drop_target {
            return;
        }
        ui::push_dense_row_inset_stroke(
            primitives,
            self.row.id(),
            bounds,
            0.5,
            Rgba8 {
                r: 255,
                g: 180,
                b: 130,
                a: 210,
            },
            1.0,
        );
    }

    pub(super) fn paint_label(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        ui::push_dense_row_label(
            primitives,
            self.row.id(),
            bounds,
            ui::DenseRowLabelParts::new(self.label.clone(), self.label_color(theme)),
        );
    }

    fn background_state_parts(&self) -> ui::InteractiveRowVisualStateParts {
        ui::InteractiveRowVisualStateParts {
            selected: self.selected,
            active_target: self.drop_target,
            candidate: self.drop_candidate,
        }
    }

    fn background_palette(&self) -> ui::DenseRowPalette {
        let background_state = self.row.dense_visual_state(self.background_state_parts());
        let interaction_fill = Rgba8 {
            r: 255,
            g: 110,
            b: 85,
            a: if background_state.pressed { 120 } else { 80 },
        };
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
            })
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
        let state = self.row.dense_visual_state(self.background_state_parts());
        state.active_target || (state.hovered && state.candidate) || state.selected
    }
}
