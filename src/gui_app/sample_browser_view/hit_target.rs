use radiant::gui::types::{Point, Rect, Rgba8};
use radiant::layout::LayoutOutput;
#[cfg(test)]
use radiant::layout::Vector2;
use radiant::prelude as ui;
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    DragHandleMessage, InteractiveRowMessage, InteractiveRowWidget, PointerModifiers, Widget,
    WidgetCommon, WidgetInput, WidgetOutput,
};

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
            .custom_paint_hit_target()
            .widget();
        Self {
            row,
            selected,
            drag_active,
            drag_source,
            cached,
            suppress_hover,
        }
    }
}

impl Widget for SampleFileHitTarget {
    fn common(&self) -> &WidgetCommon {
        self.row.common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.row.common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.row
            .handle_input_mapped(bounds, input, Self::map_row_message)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let _ = self
            .row
            .synchronize_from_previous_embedded::<Self>(previous, |previous| &previous.row);
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.paint_selection_fill(primitives, bounds);
        self.paint_interaction_fill(primitives, bounds);
        self.paint_loaded_marker(primitives, bounds);
        self.paint_selection_marker(primitives, bounds);
    }
}

impl SampleFileHitTarget {
    /// Maps generic Radiant row interactions into sample-browser hit messages.
    fn map_row_message(message: InteractiveRowMessage) -> Option<SampleFileHitMessage> {
        if let Some(modifiers) = message.activation_modifiers() {
            return Some(SampleFileHitMessage::Activate(modifiers));
        }
        if let Some(position) = message.secondary_position() {
            return Some(SampleFileHitMessage::ContextMenu(position));
        }
        message.drag_message().map(SampleFileHitMessage::Drag)
    }

    fn paint_selection_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        ui::push_dense_row_fill(
            primitives,
            self.row.id(),
            bounds,
            self.row
                .dense_visual_state(ui::InteractiveRowVisualStateParts {
                    selected: self.selected,
                    ..ui::InteractiveRowVisualStateParts::default()
                }),
            ui::DenseRowPalette::new().selected(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            }),
        );
    }

    fn paint_loaded_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.cached || self.selected {
            return;
        }
        ui::push_dense_row_vertical_marker(
            primitives,
            self.row.id(),
            bounds,
            ui::DenseRowMarkerParts::trailing(2.0),
            Rgba8 {
                r: 226,
                g: 226,
                b: 226,
                a: 210,
            },
        );
    }

    fn paint_interaction_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if self.suppress_hover {
            return;
        }
        if self.drag_active && !self.drag_source {
            return;
        }
        ui::push_dense_row_fill(
            primitives,
            self.row.id(),
            bounds,
            self.row
                .dense_visual_state(ui::InteractiveRowVisualStateParts::default()),
            ui::DenseRowPalette::new()
                .hovered(HOVER_FILL)
                .pressed(PRESSED_FILL),
        );
    }

    fn paint_selection_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        ui::push_dense_row_vertical_marker(
            primitives,
            self.row.id(),
            bounds,
            ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
            Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 245,
            },
        );
    }
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
