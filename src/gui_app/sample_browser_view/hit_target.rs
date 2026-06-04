use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::LayoutOutput;
use radiant::prelude as ui;
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{DragHandleMessage, InteractiveRowWidget, PointerModifiers};

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
pub(in crate::gui_app) struct SampleFileHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<SampleFileHitMessage>,
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
    suppress_hover: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum SampleFileHitMessage {
    Activate(PointerModifiers),
    ContextMenu(Point),
    Drag(DragHandleMessage),
}

impl SampleFileHitTarget {
    pub(in crate::gui_app) fn new(
        selected: bool,
        drag_active: bool,
        drag_source: bool,
        cached: bool,
        suppress_hover: bool,
    ) -> Self {
        let row = ui::interactive_row()
            .draggable()
            .drag_active(drag_active)
            .drag_source(drag_source)
            .suppress_hover(suppress_hover)
            .clear_hover_on_sync()
            .activation_modifiers()
            .pointer_motion_during_interaction()
            .custom_paint_hit_target()
            .widget();
        let actions = ui::InteractiveRowActions::new()
            .activate_with_modifiers(SampleFileHitMessage::Activate)
            .double_activate(|| SampleFileHitMessage::Activate(PointerModifiers::default()))
            .secondary(SampleFileHitMessage::ContextMenu)
            .drag(SampleFileHitMessage::Drag);
        Self {
            row,
            actions,
            selected,
            drag_active,
            drag_source,
            cached,
            suppress_hover,
        }
    }
}

impl ui::EmbeddedInteractiveRowWidget for SampleFileHitTarget {
    type Message = SampleFileHitMessage;

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
        let mut parts = self.row.dense_chrome_parts(
            ui::InteractiveRowVisualStateParts {
                selected: self.selected,
                ..ui::InteractiveRowVisualStateParts::default()
            },
            self.chrome_palette(),
        );
        if self.cached && !self.selected {
            parts = parts.trailing_marker(ui::DenseRowMarkerStyle::new(
                ui::DenseRowMarkerParts::trailing(2.0),
                Rgba8 {
                    r: 226,
                    g: 226,
                    b: 226,
                    a: 210,
                },
            ));
        }
        if self.selected {
            parts = parts.leading_marker(ui::DenseRowMarkerStyle::new(
                ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
                Rgba8 {
                    r: 255,
                    g: 82,
                    b: 62,
                    a: 245,
                },
            ));
        }
        parts
    }

    fn chrome_palette(&self) -> ui::DenseRowPalette {
        let mut palette = ui::DenseRowPalette::new().selected(Rgba8 {
            r: 255,
            g: 82,
            b: 62,
            a: 120,
        });
        if self.should_paint_interaction_fill() {
            palette = palette.hovered(HOVER_FILL).pressed(PRESSED_FILL);
        }
        palette
    }

    fn should_paint_interaction_fill(&self) -> bool {
        !self.suppress_hover && (!self.drag_active || self.drag_source)
    }
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
