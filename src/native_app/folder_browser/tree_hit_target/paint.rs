use radiant::{
    gui::types::{Rect, Rgba8},
    prelude as ui,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
};

use super::FolderTreeHitTarget;

impl FolderTreeHitTarget {
    pub(super) fn paint_row(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        self.row.push_dense_labeled_chrome(
            primitives,
            bounds,
            self.background_chrome_parts(),
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

    fn background_chrome_parts(&self) -> ui::DenseRowChromeParts {
        self.row
            .dense_chrome_parts(self.background_state_parts(), self.background_palette())
            .outline_if(self.drop_target, Self::drop_target_outline())
    }

    fn drop_target_outline() -> ui::DenseRowOutlineStyle {
        ui::DenseRowOutlineStyle::new(
            0.5,
            Rgba8 {
                r: 255,
                g: 180,
                b: 130,
                a: 235,
            },
            1.5,
        )
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
            .interaction_fills(interaction_fill, interaction_fill)
            .active_target(Rgba8 {
                r: 255,
                g: 130,
                b: 78,
                a: 220,
            })
            .candidate_hovered(Rgba8 {
                r: 255,
                g: 122,
                b: 74,
                a: 150,
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
        self.row
            .dense_visual_state(self.background_state_parts())
            .emphasizes_label()
    }
}
