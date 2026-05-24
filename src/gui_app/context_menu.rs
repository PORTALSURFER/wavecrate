use super::GuiMessage;
use radiant::prelude as ui;
use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{
        PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextAlign, PaintTextRun,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, PointerButton, TextWrap, Widget, WidgetCommon, WidgetInput,
        WidgetOutput, WidgetSizing,
    },
};
use std::path::{Path, PathBuf};

const CONTEXT_MENU_WIDTH: f32 = 210.0;
const CONTEXT_MENU_HEIGHT: f32 = 104.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BrowserContextTargetKind {
    Source,
    Folder,
    Sample,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserContextMenu {
    pub(super) kind: BrowserContextTargetKind,
    pub(super) path: PathBuf,
    pub(super) anchor: Point,
    pub(super) title: String,
}

pub(super) fn target_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn pane(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "sources",
        BrowserContextTargetKind::Folder => "folder_browser",
        BrowserContextTargetKind::Sample => "browser",
    }
}

pub(super) fn target_available(kind: &BrowserContextTargetKind, path: &Path) -> bool {
    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => path.is_dir(),
        BrowserContextTargetKind::Sample => path.is_file(),
    }
}

pub(super) fn missing_target_message(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "Source folder is missing",
        BrowserContextTargetKind::Folder => "Folder is missing",
        BrowserContextTargetKind::Sample => "Sample file is missing",
    }
}

pub(super) fn overlay(menu: &BrowserContextMenu) -> ui::View<GuiMessage> {
    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Sample => "Reveal in Explorer",
    };
    let top = menu.anchor.y.max(0.0);
    let left = menu.anchor.x.max(0.0);
    ui::stack([
        dismiss_area("browser-context-dismiss").fill(),
        ui::column([
            overlay_gap().fill_width().height(top),
            ui::row([
                overlay_gap().width(left).height(1.0),
                context_menu_panel(menu, action_label),
                overlay_gap().fill_width().height(1.0),
            ])
            .fill_width()
            .height(CONTEXT_MENU_HEIGHT),
            overlay_gap().fill_width().fill_height(),
        ])
        .fill(),
    ])
    .fill()
}

fn context_menu_panel(
    menu: &BrowserContextMenu,
    action_label: &'static str,
) -> ui::View<GuiMessage> {
    ui::column([
        ui::text(menu.title.clone())
            .height(22.0)
            .fill_width()
            .truncate(),
        context_menu_action(action_label, GuiMessage::OpenContextTarget)
            .key("browser-context-open-explorer")
            .fill_width()
            .height(28.0),
        context_menu_action("Copy Path", GuiMessage::CopyContextPath)
            .key("browser-context-copy-path")
            .fill_width()
            .height(28.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Neutral,
        prominence: ui::WidgetProminence::Strong,
    })
    .padding(8.0)
    .spacing(5.0)
    .width(CONTEXT_MENU_WIDTH)
    .height(CONTEXT_MENU_HEIGHT)
}

fn context_menu_action(label: impl Into<String>, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::custom_widget_mapped(
        ContextMenuActionButton::new(label.into(), message),
        |message: GuiMessage| message,
    )
}

fn overlay_gap() -> ui::View<GuiMessage> {
    ui::text("")
}

fn dismiss_area(key: &'static str) -> ui::View<GuiMessage> {
    ui::button("")
        .message(GuiMessage::CloseContextMenu)
        .key(key)
        .input_only()
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
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(self.message.clone()))
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
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
        let hovered = self.common.state.hovered || self.common.state.pressed;
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: context_menu_button_fill(),
        }));
        if hovered {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: bounds,
                color: context_menu_hover_fill(self.common.state.pressed),
            }));
        }
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 0.5, bounds.min.y + 0.5),
                Point::new(bounds.max.x - 0.5, bounds.max.y - 0.5),
            ),
            color: context_menu_button_border(),
            width: 1.0,
        }));

        let text_rect = Rect::from_min_max(
            Point::new(bounds.min.x + 8.0, bounds.min.y),
            Point::new(bounds.max.x - 8.0, bounds.max.y),
        );
        let font_size = 11.0;
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: PaintText::from(self.label.as_str()),
            rect: text_rect,
            font_size,
            baseline: Some(((text_rect.height() - font_size) * 0.5 + font_size * 0.78).round()),
            color: context_menu_text(),
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
        }));
    }
}

fn context_menu_button_fill() -> Rgba8 {
    Rgba8 {
        r: 30,
        g: 31,
        b: 31,
        a: 235,
    }
}

fn context_menu_button_border() -> Rgba8 {
    Rgba8 {
        r: 66,
        g: 67,
        b: 68,
        a: 175,
    }
}

fn context_menu_hover_fill(pressed: bool) -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 108,
        b: 88,
        a: if pressed { 170 } else { 155 },
    }
}

fn context_menu_text() -> Rgba8 {
    Rgba8 {
        r: 218,
        g: 219,
        b: 219,
        a: 238,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                PaintPrimitive::FillRect(fill) if fill.color == context_menu_hover_fill(false)
            )),
            "{default_primitives:?}"
        );
        assert!(default_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.color == context_menu_text()
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
                if fill.rect == bounds && fill.color == context_menu_hover_fill(false)
        )));
        assert!(hover_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.color == context_menu_text()
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
