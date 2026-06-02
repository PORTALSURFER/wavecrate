use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{
        DragHandleMessage, InteractiveRowMessage, InteractiveRowWidget, Widget, WidgetCommon,
        WidgetInput, WidgetOutput,
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
        let row = ui::interactive_row()
            .draggable()
            .drag_active(drag_active)
            .drag_source(drag_source)
            .drag_source_motion(true)
            .pointer_motion_during_interaction()
            .pointer_motion_active(drop_target_active)
            .drop_target_mode(
                drag_active && !drag_source,
                drop_hover_enabled(drop_target, drop_candidate, drop_target_active),
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

impl Widget for FolderTreeHitTarget {
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

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let _ = self
            .row
            .synchronize_from_previous_embedded::<Self>(previous, |previous| &previous.row);
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

fn drop_hover_enabled(drop_target: bool, drop_candidate: bool, drop_target_active: bool) -> bool {
    !drop_target && (drop_candidate || drop_target_active)
}

#[cfg(test)]
mod tests;
