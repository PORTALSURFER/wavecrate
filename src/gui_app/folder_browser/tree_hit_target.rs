use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds,
        Widget, WidgetCommon, WidgetId, WidgetInput, WidgetOutput,
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
    drop_candidate: bool,
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
        let mut row = ui::interactive_row()
            .draggable()
            .drag_active(drag_active)
            .drag_source(drag_source)
            .drag_source_motion(true)
            .pointer_motion_during_interaction()
            .pointer_motion_active(drop_target_active);
        if drag_active && !drag_source {
            row = if drop_hover_enabled(drop_target, drop_candidate, drop_target_active) {
                row.droppable(true)
            } else {
                row.drop_only(true)
            };
        }
        let row = row
            .focus(FocusBehavior::None)
            .paint_bounds(PaintBounds::ClipToRect)
            .paint_focus(false)
            .paint_state_layers(false)
            .widget();
        Self {
            row,
            label: label.into(),
            selected,
            drop_target,
            drop_candidate,
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
        self.row
            .handle_input(bounds, input)
            .and_then(|message| self.map_row_message(message))
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.row.synchronize_from_previous(&previous.row);
    }

    fn accepts_pointer_move(&self) -> bool {
        self.row.accepts_pointer_move()
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
    fn map_row_message(&self, message: InteractiveRowMessage) -> Option<FolderTreeHitMessage> {
        match message {
            InteractiveRowMessage::Activate
            | InteractiveRowMessage::ActivateWithModifiers { .. }
            | InteractiveRowMessage::DoubleActivate => Some(FolderTreeHitMessage::Activate),
            InteractiveRowMessage::SecondaryActivate { position } => {
                Some(FolderTreeHitMessage::ContextMenu(position))
            }
            InteractiveRowMessage::Drag(message) => Some(FolderTreeHitMessage::Drag(message)),
            InteractiveRowMessage::Drop => Some(FolderTreeHitMessage::Drop),
            InteractiveRowMessage::HoverDropTarget { position } => {
                Some(FolderTreeHitMessage::HoverDropTarget(position))
            }
        }
    }
}

fn drop_hover_enabled(drop_target: bool, drop_candidate: bool, drop_target_active: bool) -> bool {
    !drop_target && (drop_candidate || drop_target_active)
}

#[cfg(test)]
mod tests;
