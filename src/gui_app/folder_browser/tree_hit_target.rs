use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

mod paint;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum FolderTreeHitMessage {
    Activate,
    ContextMenu(Point),
    Drag(DragHandleMessage),
    Drop,
    HoverDropTarget(Point),
}

#[derive(Clone, Debug)]
pub(super) struct FolderTreeHitTarget {
    row: InteractiveRowWidget,
    label: PaintText,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    drag_source: bool,
    drop_candidate: bool,
    drop_target_active: bool,
}

impl FolderTreeHitTarget {
    pub(super) fn new(
        label: impl Into<PaintText>,
        selected: bool,
        drop_target: bool,
        drag_active: bool,
        drag_source: bool,
        drop_candidate: bool,
        drop_target_active: bool,
    ) -> Self {
        let mut row = InteractiveRowWidget::new(0, WidgetSizing::fixed(Vector2::new(1.0, 22.0)))
            .with_drag()
            .with_drag_active(drag_active)
            .with_drag_source(drag_source)
            .with_drag_source_motion(true);
        if drag_active && !drag_source {
            row = if drop_hover_enabled(drop_target, drop_candidate, drop_target_active) {
                row.with_drop_target(true)
            } else {
                row.with_drop_only(true)
            };
        }
        row.common.focus = FocusBehavior::None;
        row.common.paint.bounds = PaintBounds::ClipToRect;
        row.common.paint.paints_focus = false;
        row.common.paint.paints_state_layers = false;
        Self {
            row,
            label: label.into(),
            selected,
            drop_target,
            drag_active,
            drag_source,
            drop_candidate,
            drop_target_active,
        }
    }
}

impl Widget for FolderTreeHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.row.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.row.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let move_position = pointer_move_position(&input);
        self.row
            .handle_input(bounds, input)
            .and_then(|message| self.map_row_message(message, move_position))
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.row.synchronize_from_previous(&previous.row);
    }

    fn accepts_pointer_move(&self) -> bool {
        self.row.common.state.pressed
            || self.drag_active
            || self.drag_source
            || self.drop_target_active
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.paint_background(primitives, bounds);
        self.paint_drop_target_outline(primitives, bounds);
        self.paint_label(primitives, bounds, _theme);
    }
}

impl FolderTreeHitTarget {
    fn map_row_message(
        &self,
        message: InteractiveRowMessage,
        move_position: Option<Point>,
    ) -> Option<FolderTreeHitMessage> {
        match message {
            InteractiveRowMessage::Activate | InteractiveRowMessage::DoubleActivate => {
                Some(FolderTreeHitMessage::Activate)
            }
            InteractiveRowMessage::SecondaryActivate { position } => {
                Some(FolderTreeHitMessage::ContextMenu(position))
            }
            InteractiveRowMessage::Drag(message) => Some(FolderTreeHitMessage::Drag(message)),
            InteractiveRowMessage::Drop => Some(FolderTreeHitMessage::Drop),
            InteractiveRowMessage::HoverDropTarget => {
                move_position.map(FolderTreeHitMessage::HoverDropTarget)
            }
        }
    }
}

fn pointer_move_position(input: &WidgetInput) -> Option<Point> {
    match input {
        WidgetInput::PointerMove { position } => Some(*position),
        _ => None,
    }
}

fn drop_hover_enabled(drop_target: bool, drop_candidate: bool, drop_target_active: bool) -> bool {
    !drop_target && (drop_candidate || drop_target_active)
}

#[cfg(test)]
mod tests;
