use radiant::{
    gui::types::{Point, Rect},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{DragHandleMessage, InteractiveRowMessage, InteractiveRowWidget},
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
        let row = ui::interactive_row()
            .draggable()
            .drag_active(drag_active)
            .drag_source(drag_source)
            .drag_source_motion(true)
            .pointer_motion_during_interaction()
            .tracked_drop_candidate(
                drag_active && !drag_source,
                drop_target,
                drop_candidate,
                drop_target_active,
            )
            .custom_paint_hit_target()
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

impl ui::EmbeddedInteractiveRowWidget for FolderTreeHitTarget {
    type Message = FolderTreeHitMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn map_interactive_row_message(message: InteractiveRowMessage) -> Option<Self::Message> {
        Self::map_row_message(message)
    }

    fn append_interactive_row_paint(
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
    fn map_row_message(message: InteractiveRowMessage) -> Option<FolderTreeHitMessage> {
        if message.is_activation() {
            return Some(FolderTreeHitMessage::Activate);
        }
        if let Some(position) = message.secondary_position() {
            return Some(FolderTreeHitMessage::ContextMenu(position));
        }
        if let Some(message) = message.drag_message() {
            return Some(FolderTreeHitMessage::Drag(message));
        }
        if let Some(position) = message.hover_drop_position() {
            return Some(FolderTreeHitMessage::HoverDropTarget(position));
        }
        message.is_drop().then_some(FolderTreeHitMessage::Drop)
    }
}

#[cfg(test)]
mod tests;
