use super::super::GuiMessage;
mod paint;

use radiant::prelude as ui;
use radiant::{
    gui::types::Rect,
    layout::{LayoutOutput, Vector2},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

pub(super) fn view(label: impl Into<String>, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::custom_widget_mapped(
        ContextMenuActionButton::new(label.into(), message),
        |message: GuiMessage| message,
    )
}

#[derive(Clone, Debug)]
struct ContextMenuActionButton {
    common: WidgetCommon,
    label: String,
    message: GuiMessage,
}

impl ContextMenuActionButton {
    fn new(label: String, message: GuiMessage) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(1.0, 28.0)));
        common.focus = FocusBehavior::None;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            label,
            message,
        }
    }
}

impl Widget for ContextMenuActionButton {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ui::handle_activation_input(
            &mut self.common.state,
            bounds,
            &input,
            ui::ActivationInputPolicy::pointer_only(),
        )
        .activated()
        .then(|| WidgetOutput::typed(self.message.clone()))
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        paint::background(primitives, self.common.id, bounds);
        if self.common.state.hovered || self.common.state.pressed {
            paint::hover(
                primitives,
                self.common.id,
                bounds,
                self.common.state.pressed,
            );
        }
        paint::border(primitives, self.common.id, bounds);
        paint::label(primitives, self.common.id, bounds, &self.label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::gui::types::Point;
    use radiant::widgets::PointerButton;
    use radiant::widgets::PointerModifiers;

    fn action_primitives(button: &ContextMenuActionButton) -> Vec<PaintPrimitive> {
        let mut primitives = Vec::new();
        button.append_paint(
            &mut primitives,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 28.0)),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        primitives
    }

    #[test]
    fn action_button_uses_hover_fill_without_recoloring_text() {
        let mut button =
            ContextMenuActionButton::new(String::from("Copy Path"), GuiMessage::CopyContextPath);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 28.0));

        let default_primitives = action_primitives(&button);
        assert!(
            !default_primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::FillRect(fill) if fill.color == paint::hover_fill(false)
            )),
            "{default_primitives:?}"
        );
        assert!(default_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.color == paint::text_color()
        )));

        button.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(20.0, 12.0),
            },
        );

        let hover_primitives = action_primitives(&button);
        assert!(hover_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect == bounds && fill.color == paint::hover_fill(false)
        )));
        assert!(hover_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.color == paint::text_color()
        )));
    }

    #[test]
    fn action_button_emits_configured_message_on_click() {
        let mut button =
            ContextMenuActionButton::new(String::from("Copy Path"), GuiMessage::CopyContextPath);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 28.0));

        button.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(20.0, 12.0),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        let output = button
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(20.0, 12.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )
            .expect("click should emit action message");

        assert_eq!(
            output.typed_ref::<GuiMessage>(),
            Some(&GuiMessage::CopyContextPath)
        );
    }
}
