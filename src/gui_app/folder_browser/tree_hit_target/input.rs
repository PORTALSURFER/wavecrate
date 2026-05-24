use super::*;

impl FolderTreeHitTarget {
    pub(super) fn handle_pointer_move(
        &mut self,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        if self.common.state.pressed || self.drag_source {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(
                self.next_drag_move(position),
            )));
        }
        if self.should_report_drop_hover() {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::HoverDropTarget(
                position,
            )));
        }
        None
    }

    pub(super) fn handle_primary_release(
        &mut self,
        bounds: Rect,
        position: Point,
    ) -> Option<WidgetOutput> {
        if self.release_drops_on_target(bounds, position) {
            self.clear_press_after_release(bounds, position);
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drop));
        }
        let activated = self.common.state.pressed && !self.dragged && bounds.contains(position);
        let dragged = self.release_ends_drag();
        self.clear_press_after_release(bounds, position);
        if dragged {
            return Some(WidgetOutput::typed(FolderTreeHitMessage::Drag(
                DragHandleMessage::Ended { position },
            )));
        }
        activated.then(|| WidgetOutput::typed(FolderTreeHitMessage::Activate))
    }

    fn next_drag_move(&mut self, position: Point) -> DragHandleMessage {
        if self.dragged || self.drag_active {
            DragHandleMessage::Moved { position }
        } else {
            self.dragged = true;
            DragHandleMessage::Started { position }
        }
    }

    fn should_report_drop_hover(&self) -> bool {
        self.common.state.hovered
            && self.drag_active
            && !self.drop_target
            && (self.drop_candidate || self.drop_target_active)
    }

    fn release_drops_on_target(&self, bounds: Rect, position: Point) -> bool {
        self.drag_active && !self.drag_source && bounds.contains(position)
    }

    fn release_ends_drag(&self) -> bool {
        self.drag_source || (self.common.state.pressed && (self.dragged || self.drag_active))
    }

    fn clear_press_after_release(&mut self, bounds: Rect, position: Point) {
        self.common.state.pressed = false;
        self.common.state.hovered = bounds.contains(position);
        self.dragged = false;
    }
}
