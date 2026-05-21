use radiant::{
    gui::types::Rect,
    layout::{LayoutOutput, Vector2},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

use super::FolderBrowserMessage;

#[derive(Clone, Debug)]
pub(super) struct FolderDropClearTarget {
    common: WidgetCommon,
    drag_active: bool,
}

impl FolderDropClearTarget {
    pub(super) fn new(drag_active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            drag_active,
        }
    }
}

impl Widget for FolderDropClearTarget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position }
                if self.drag_active && bounds.contains(position) =>
            {
                Some(WidgetOutput::typed(FolderBrowserMessage::ClearDropTarget))
            }
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}
