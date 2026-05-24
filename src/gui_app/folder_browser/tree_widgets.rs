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
    drop_target_active: bool,
}

impl FolderDropClearTarget {
    pub(super) fn new(drop_target_active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 1.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            drop_target_active,
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
                if self.drop_target_active && bounds.contains(position) =>
            {
                Some(WidgetOutput::typed(FolderBrowserMessage::ClearDropTarget(
                    position,
                )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::gui::types::Point;

    #[test]
    fn clear_target_reports_hover_when_drop_target_is_active() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 80.0));
        let mut target = FolderDropClearTarget::new(true);
        let output = target
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(30.0, 12.0),
                },
            )
            .expect("clear target should emit pointer position");
        assert_eq!(
            output.typed_ref::<FolderBrowserMessage>(),
            Some(&FolderBrowserMessage::ClearDropTarget(Point::new(
                30.0, 12.0
            ))),
            "drop hover clearing must not wait for a refreshed drag_active projection"
        );
    }

    #[test]
    fn clear_target_stays_quiet_when_no_drop_target_is_active() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 80.0));
        let mut target = FolderDropClearTarget::new(false);

        assert!(
            target
                .handle_input(
                    bounds,
                    WidgetInput::PointerMove {
                        position: Point::new(30.0, 12.0),
                    },
                )
                .is_none(),
            "background pointer motion should not force scene rebuilds when there is no drop target to clear"
        );
    }
}
