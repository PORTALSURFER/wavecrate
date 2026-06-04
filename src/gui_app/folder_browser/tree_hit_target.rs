use radiant::{
    gui::types::{Point, Rect},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{DragHandleMessage, InteractiveRowWidget},
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
    actions: ui::InteractiveRowActions<FolderTreeHitMessage>,
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
            .tracked_drop_candidate(
                drag_active && !drag_source,
                drop_target,
                drop_candidate,
                drop_target_active,
            )
            .custom_paint_hit_target();
        if drag_active || drop_target_active {
            row = row.clear_hover_on_sync();
        }
        let row = row.widget();
        let actions = ui::InteractiveRowActions::new()
            .activate(|| FolderTreeHitMessage::Activate)
            .double_activate(|| FolderTreeHitMessage::Activate)
            .secondary(FolderTreeHitMessage::ContextMenu)
            .drag(FolderTreeHitMessage::Drag)
            .hover_drop(FolderTreeHitMessage::HoverDropTarget)
            .drop(|| FolderTreeHitMessage::Drop);
        Self {
            row,
            actions,
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
        self.paint_background(primitives, bounds);
        self.paint_label(primitives, bounds, _theme);
    }
}

#[cfg(test)]
mod tests;
