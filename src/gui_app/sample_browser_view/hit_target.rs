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
        &self.row.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.row.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.row
            .handle_input(bounds, input)
            .and_then(|message| self.map_row_message(message))
            .map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.row.synchronize_from_previous(&previous.row);
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
    fn map_row_message(&self, message: InteractiveRowMessage) -> Option<SampleFileHitMessage> {
        match message {
            InteractiveRowMessage::Activate => {
                Some(SampleFileHitMessage::Activate(PointerModifiers::default()))
            }
            InteractiveRowMessage::ActivateWithModifiers { modifiers } => {
                Some(SampleFileHitMessage::Activate(modifiers))
            }
            InteractiveRowMessage::DoubleActivate => {
                Some(SampleFileHitMessage::Activate(PointerModifiers::default()))
            }
            InteractiveRowMessage::SecondaryActivate { position } => {
                Some(SampleFileHitMessage::ContextMenu(position))
            }
            InteractiveRowMessage::Drag(message) => Some(SampleFileHitMessage::Drag(message)),
            InteractiveRowMessage::Drop | InteractiveRowMessage::HoverDropTarget { .. } => None,
        }
    }

    fn paint_selection_fill(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        let Some(color) = ui::dense_row_fill_color(
            ui::DenseRowVisualState {
                selected: self.selected,
                ..ui::DenseRowVisualState::default()
            },
            ui::DenseRowPalette::new().selected(Rgba8 {
                r: 255,
                g: 82,
                b: 62,
                a: 120,
            }),
        ) else {
            return;
        };
        ui::push_fill_rect(primitives, self.row.common.id, bounds, color);
    }

    fn paint_loaded_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.cached || self.selected {
            return;
        }
        let Some(rect) = ui::dense_row_vertical_marker_rect(
            bounds,
            ui::DenseRowMarkerParts {
                edge: ui::DenseRowMarkerEdge::Trailing,
                width: 2.0,
                edge_inset: 1.0,
                vertical_inset: 3.0,
                min_height: 8.0,
            },
        ) else {
            return;
        };
        ui::push_fill_rect(
            primitives,
            self.row.common.id,
            rect,
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
        let Some(color) = ui::dense_row_fill_color(
            ui::DenseRowVisualState {
                hovered: self.row.common.state.hovered,
                pressed: self.row.common.state.pressed,
                ..ui::DenseRowVisualState::default()
            },
            ui::DenseRowPalette::new()
                .hovered(HOVER_FILL)
                .pressed(PRESSED_FILL),
        ) else {
            return;
        };
        ui::push_fill_rect(primitives, self.row.common.id, bounds, color);
    }

    fn paint_selection_marker(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.selected {
            return;
        }
        let Some(rect) = ui::dense_row_vertical_marker_rect(
            bounds,
            ui::DenseRowMarkerParts {
                edge: ui::DenseRowMarkerEdge::Leading,
                width: 3.0,
                edge_inset: 1.0,
                vertical_inset: 4.0,
                min_height: 8.0,
            },
        ) else {
            return;
        };
        ui::push_fill_rect(
            primitives,
            self.row.common.id,
            rect,
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
