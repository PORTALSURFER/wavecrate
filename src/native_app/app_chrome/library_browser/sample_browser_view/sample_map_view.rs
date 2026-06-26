use radiant::{
    gui::types::{Point, Rect, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, push_fill_rect, push_stroke_rect},
    theme::ThemeTokens,
    widgets::{
        CanvasGestureEvent, CanvasGestureState, PointerButton, PointerModifiers, Widget,
        WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::folder_browser::sample_map::SampleMapItem;
use crate::native_app::ui::ids as widget_ids;

const MAP_MIN_HEIGHT: f32 = 240.0;
const MAP_NODE_SIZE: f32 = 4.0;
const MAP_SELECTED_SIZE: f32 = 8.0;
const MAP_ANCHOR_SIZE: f32 = 10.0;
const MAP_HIT_RADIUS: f32 = 8.0;

pub(super) fn sample_map_view(items: Vec<SampleMapItem>) -> ui::View<GuiMessage> {
    if items.is_empty() {
        return ui::column([
            ui::text_line("No audio files in selected folder", 23.0).muted_text(),
            ui::spacer().fill_height(),
        ])
        .spacing(0.0)
        .fill();
    }

    ui::custom_widget_direct(SampleMapWidget::new(items))
        .id(widget_ids::SAMPLE_BROWSER_MAP_ID)
        .height(MAP_MIN_HEIGHT)
        .fill()
}

#[derive(Clone, Debug)]
struct SampleMapWidget {
    common: WidgetCommon,
    gesture: CanvasGestureState,
    items: Vec<SampleMapItem>,
    last_hit_file_id: Option<String>,
}

impl SampleMapWidget {
    fn new(items: Vec<SampleMapItem>) -> Self {
        let common = WidgetCommon::new(
            widget_ids::SAMPLE_BROWSER_MAP_ID,
            WidgetSizing::new(
                Vector2::new(1.0, MAP_MIN_HEIGHT),
                Vector2::new(640.0, 360.0),
            ),
        )
        .with_pointer_focus()
        .without_default_chrome();
        Self {
            common,
            gesture: CanvasGestureState::new(),
            items,
            last_hit_file_id: None,
        }
    }

    fn message_for_hit(
        &mut self,
        bounds: Rect,
        point: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let hit_file_id = self.hit_test(bounds, point)?.file_id.clone();
        if self.last_hit_file_id.as_deref() == Some(hit_file_id.as_str()) {
            return None;
        }
        self.last_hit_file_id = Some(hit_file_id.clone());
        Some(WidgetOutput::typed(GuiMessage::SelectSampleWithModifiers {
            path: hit_file_id,
            modifiers,
        }))
    }

    fn hit_test(&self, bounds: Rect, point: Point) -> Option<&SampleMapItem> {
        let mut best: Option<(&SampleMapItem, f32)> = None;
        for item in &self.items {
            let center = item_center(bounds, item);
            let distance_sq = distance_squared(center, point);
            if distance_sq > MAP_HIT_RADIUS * MAP_HIT_RADIUS {
                continue;
            }
            if best.is_none_or(|(_, best_distance)| distance_sq < best_distance) {
                best = Some((item, distance_sq));
            }
        }
        best.map(|(item, _)| item)
    }
}

impl Widget for SampleMapWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let event = self.gesture.handle_input(bounds, &input)?;
        match event {
            CanvasGestureEvent::Press {
                pointer,
                button: PointerButton::Primary,
                modifiers,
            } => self.message_for_hit(bounds, pointer.position, modifiers),
            CanvasGestureEvent::Drag {
                pointer,
                button: PointerButton::Primary,
                modifiers,
                ..
            } => self.message_for_hit(bounds, pointer.position, modifiers),
            CanvasGestureEvent::Release { .. } | CanvasGestureEvent::Drop { .. } => {
                self.last_hit_file_id = None;
                None
            }
            _ => None,
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        push_fill_rect(
            primitives,
            self.common.id,
            bounds,
            ui::Rgba8::new(8, 9, 10, 255),
        );
        for item in &self.items {
            paint_item(primitives, self.common.id, bounds, item);
        }
    }
}

fn paint_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    item: &SampleMapItem,
) {
    let center = item_center(bounds, item);
    let size = if item.similarity_anchor {
        MAP_ANCHOR_SIZE
    } else if item.selected {
        MAP_SELECTED_SIZE
    } else {
        MAP_NODE_SIZE
    };
    let color = if item.missing {
        ui::Rgba8::new(120, 120, 120, 180)
    } else {
        item.color
    };
    let rect = centered_rect(center, size);
    push_fill_rect(primitives, widget_id, rect, color);
    if item.selected || item.similarity_anchor {
        push_stroke_rect(
            primitives,
            widget_id,
            centered_rect(center, size + 4.0),
            ui::Rgba8::new(245, 245, 245, 220),
            1.0,
        );
    }
}

fn item_center(bounds: Rect, item: &SampleMapItem) -> Point {
    Point::new(bounds.x_for_ratio(item.x), bounds.y_for_ratio(item.y))
}

fn centered_rect(center: Point, side: f32) -> Rect {
    Rect::from_xy_size(center.x - side * 0.5, center.y - side * 0.5, side, side)
}

fn distance_squared(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}
