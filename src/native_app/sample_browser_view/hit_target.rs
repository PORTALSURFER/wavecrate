use radiant::gui::types::{Rect, Rgba8};
use radiant::layout::LayoutOutput;
use radiant::prelude as ui;
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::InteractiveRowWidget;

use crate::native_app::app_scope::GuiMessage;

const HOVER_FILL: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 24,
};
const PRESSED_FILL: Rgba8 = Rgba8 {
    r: 255,
    g: 108,
    b: 88,
    a: 170,
};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SampleFileHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<GuiMessage>,
    selected: bool,
    cached: bool,
}

impl SampleFileHitTarget {
    pub(in crate::native_app) fn new(
        path: String,
        selected: bool,
        drag_active: bool,
        drag_source: bool,
        cached: bool,
        suppress_hover: bool,
    ) -> Self {
        let row = ui::interactive_row()
            .tracked_drag_source(drag_active, drag_source)
            .suppress_hover(suppress_hover)
            .clear_hover_on_sync()
            .activation_modifiers()
            .custom_paint_hit_target()
            .widget();
        let actions = ui::InteractiveRowActions::new()
            .activate_or_double_with_modifiers_secondary_drag_key(
                path,
                |path, modifiers| GuiMessage::SelectSampleWithModifiers { path, modifiers },
                |path, position| GuiMessage::OpenSampleContextMenu { path, position },
                |path, drag| GuiMessage::DragSampleFile { path, drag },
            );
        Self {
            row,
            actions,
            selected,
            cached,
        }
    }
}

impl ui::EmbeddedInteractiveRowWidget for SampleFileHitTarget {
    type Message = GuiMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn interactive_row_actions(&self) -> Option<&ui::InteractiveRowActions<Self::Message>> {
        Some(&self.actions)
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.row
            .push_dense_chrome(primitives, bounds, self.chrome_parts());
    }
}

impl SampleFileHitTarget {
    fn chrome_parts(&self) -> ui::DenseRowChromeParts {
        self.row
            .dense_chrome_parts(
                ui::InteractiveRowVisualStateParts {
                    selected: self.selected,
                    ..ui::InteractiveRowVisualStateParts::default()
                },
                self.chrome_palette(),
            )
            .trailing_marker_if(
                self.cached && !self.selected,
                ui::DenseRowMarkerStyle::new(
                    ui::DenseRowMarkerParts::trailing(2.0),
                    Rgba8 {
                        r: 226,
                        g: 226,
                        b: 226,
                        a: 210,
                    },
                ),
            )
            .leading_marker_if(
                self.selected,
                ui::DenseRowMarkerStyle::new(
                    ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
                    Rgba8 {
                        r: 255,
                        g: 82,
                        b: 62,
                        a: 245,
                    },
                ),
            )
    }

    fn chrome_palette(&self) -> ui::DenseRowPalette {
        ui::DenseRowPalette::new()
            .selected(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            })
            .interaction_fills_if(self.row.paints_interaction_fill(), HOVER_FILL, PRESSED_FILL)
    }
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
