use radiant::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, Vector2},
    runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintText, PaintTextRun},
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, InteractiveRowMessage, InteractiveRowWidget, PaintBounds, TextWrap, Widget,
        WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

use super::{COLLECTION_ROW_HEIGHT, SampleCollectionView};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) enum CollectionHitMessage {
    Activate,
    Rename,
    Drop,
    HoverDropTarget(Point),
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct CollectionHitTarget {
    row: InteractiveRowWidget,
    label: String,
    hotkey: char,
    color: Rgba8,
    selected: bool,
    drop_target: bool,
    drag_active: bool,
    assigned_count: usize,
}

impl CollectionHitTarget {
    pub(in crate::gui_app) fn new(collection: &SampleCollectionView) -> Self {
        let mut row = InteractiveRowWidget::new(
            0,
            WidgetSizing::fixed(Vector2::new(1.0, COLLECTION_ROW_HEIGHT)),
        );
        row = if collection.drag_active && !collection.drop_target {
            row.with_drop_target(true)
        } else if collection.drag_active {
            row.with_drop_only(true)
        } else {
            row
        };
        row.common.focus = FocusBehavior::None;
        row.common.paint.bounds = PaintBounds::ClipToRect;
        row.common.paint.paints_focus = false;
        row.common.paint.paints_state_layers = false;
        Self {
            row,
            label: collection.name.clone(),
            hotkey: collection.hotkey,
            color: collection.color,
            selected: collection.selected,
            drop_target: collection.drop_target,
            drag_active: collection.drag_active,
            assigned_count: collection.assigned_count,
        }
    }
}

impl Widget for CollectionHitTarget {
    fn common(&self) -> &WidgetCommon {
        &self.row.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.row.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.row
            .handle_input(bounds, input)
            .and_then(map_collection_row_message)
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.row.synchronize_from_previous(&previous.row);
    }

    fn accepts_pointer_move(&self) -> bool {
        self.drag_active || self.drop_target || self.row.common.state.pressed
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.paint_background(primitives, bounds, theme);
        self.paint_drop_target(primitives, bounds);
        self.paint_swatch(primitives, bounds);
        self.paint_label(primitives, bounds, theme);
        self.paint_assigned_count(primitives, bounds, theme);
    }
}

impl CollectionHitTarget {
    fn paint_background(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        if !(self.selected || self.row.common.state.hovered || self.drop_target) {
            return;
        }

        let color = if self.drop_target {
            self.color.blend_toward(theme.bg_primary, 0.72)
        } else if self.selected {
            theme.accent_mint.blend_toward(theme.bg_primary, 0.82)
        } else {
            theme.bg_secondary.blend_toward(theme.text_primary, 0.10)
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: bounds,
            color,
        }));
    }

    fn paint_drop_target(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if !self.drop_target {
            return;
        }

        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.row.common.id,
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 1.0, bounds.min.y + 1.0),
                Point::new(bounds.max.x - 1.0, bounds.max.y - 1.0),
            ),
            color: self.color,
            width: 1.0,
        }));
    }

    fn paint_swatch(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        let swatch = Rect::from_min_size(
            Point::new(bounds.min.x + 6.0, bounds.min.y + 6.0),
            Vector2::new(10.0, 10.0),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.row.common.id,
            rect: swatch,
            color: self.color,
        }));
    }

    fn paint_label(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect, theme: &ThemeTokens) {
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.row.common.id,
            text: PaintText::from(format!("{}  {}", self.hotkey, self.label)),
            rect: Rect::from_min_max(
                Point::new(bounds.min.x + 22.0, bounds.min.y),
                Point::new(bounds.max.x - 38.0, bounds.max.y),
            ),
            font_size: 12.0,
            baseline: Some((bounds.height() * 0.5 + 12.0 * 0.35).max(0.0)),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Left,
            wrap: TextWrap::None,
        }));
    }

    fn paint_assigned_count(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
    ) {
        if self.assigned_count == 0 {
            return;
        }

        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.row.common.id,
            text: PaintText::from(self.assigned_count.to_string()),
            rect: Rect::from_min_max(
                Point::new(bounds.max.x - 34.0, bounds.min.y),
                Point::new(bounds.max.x - 6.0, bounds.max.y),
            ),
            font_size: 11.0,
            baseline: Some((bounds.height() * 0.5 + 11.0 * 0.35).max(0.0)),
            color: theme.text_muted,
            align: radiant::runtime::PaintTextAlign::Right,
            wrap: TextWrap::None,
        }));
    }
}

fn map_collection_row_message(message: InteractiveRowMessage) -> Option<CollectionHitMessage> {
    match message {
        InteractiveRowMessage::Activate => Some(CollectionHitMessage::Activate),
        InteractiveRowMessage::DoubleActivate => Some(CollectionHitMessage::Rename),
        InteractiveRowMessage::Drop => Some(CollectionHitMessage::Drop),
        InteractiveRowMessage::HoverDropTarget { position } => {
            Some(CollectionHitMessage::HoverDropTarget(position))
        }
        InteractiveRowMessage::SecondaryActivate { .. } | InteractiveRowMessage::Drag(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::widgets::{PointerButton, PointerModifiers};
    use wavecrate::sample_sources::SampleCollection;

    fn collection_view(drag_active: bool, drop_target: bool) -> SampleCollectionView {
        SampleCollectionView {
            collection: SampleCollection::new(0).expect("valid collection"),
            hotkey: '1',
            name: String::from("Collection 1"),
            color: Rgba8 {
                r: 255,
                g: 86,
                b: 98,
                a: 240,
            },
            selected: false,
            drop_target,
            drag_active,
            assigned_count: 0,
        }
    }

    fn message_from(output: Option<WidgetOutput>) -> CollectionHitMessage {
        output
            .expect("expected collection output")
            .typed_ref::<CollectionHitMessage>()
            .expect("expected collection message")
            .clone()
    }

    #[test]
    fn double_click_requests_collection_rename() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = CollectionHitTarget::new(&collection_view(false, false));

        assert_eq!(
            message_from(target.handle_input(
                bounds,
                WidgetInput::PointerDoubleClick {
                    position: Point::new(12.0, 8.0),
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )),
            CollectionHitMessage::Rename
        );
    }

    #[test]
    fn drag_hover_reports_collection_drop_target() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
        let mut target = CollectionHitTarget::new(&collection_view(true, false));

        assert_eq!(
            message_from(target.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(18.0, 8.0),
                },
            )),
            CollectionHitMessage::HoverDropTarget(Point::new(18.0, 8.0))
        );
    }
}
