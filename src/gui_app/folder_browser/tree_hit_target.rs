use radiant::{
    gui::types::Rect,
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::InteractiveRowWidget,
};

use crate::gui_app::{FolderBrowserMessage, GuiMessage};

mod paint;

#[derive(Clone, Debug)]
pub(super) struct FolderTreeHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<GuiMessage>,
    label: PaintText,
    selected: bool,
    drop_target: bool,
    drop_candidate: bool,
}

impl FolderTreeHitTarget {
    pub(super) fn new(
        id: String,
        label: impl Into<PaintText>,
        selected: bool,
        drop_target: bool,
        drag_active: bool,
        drag_source: bool,
        drop_candidate: bool,
        drop_target_active: bool,
    ) -> Self {
        let mut row = ui::interactive_row()
            .tracked_drag_source_with_motion(drag_active, drag_source)
            .tracked_drop_candidate(
                drag_active && !drag_source,
                drop_target,
                drop_candidate,
                drop_target_active,
            )
            .custom_paint_hit_target();
        if (drag_active || drop_target_active) && !drop_target {
            row = row.clear_hover_on_sync();
        }
        let row = row.widget();
        let actions = ui::InteractiveRowActions::new()
            .activate_or_double_secondary_drag_drop_target_key(
                id,
                |id| GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateFolder(id)),
                |id, position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::OpenFolderContextMenu(
                        id, position,
                    ))
                },
                |id, drag| GuiMessage::FolderBrowser(FolderBrowserMessage::DragFolder(id, drag)),
                |id| GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnFolder(id)),
                move |id, position| {
                    let message = if drop_target_active && !drop_candidate {
                        FolderBrowserMessage::ClearDropTargetUnless(id, position)
                    } else {
                        FolderBrowserMessage::HoverDropTarget(id, position)
                    };
                    GuiMessage::FolderBrowser(message)
                },
            );
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
        self.paint_row(primitives, bounds, _theme);
    }
}

#[cfg(test)]
mod tests;
