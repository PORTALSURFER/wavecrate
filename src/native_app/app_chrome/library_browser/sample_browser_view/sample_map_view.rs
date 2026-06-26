use radiant::{
    gui::types::{Point, Rect, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, push_fill_rect, push_stroke_rect},
    theme::ThemeTokens,
    widgets::{
        CanvasGestureEvent, CanvasGestureState, PointerButton, PointerModifiers, TextInputMessage,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};

use crate::native_app::app::{GuiMessage, SampleMapViewport, SampleMapViewportChange};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::sample_map::SampleMapItem;
use crate::native_app::ui::ids as widget_ids;

const MAP_MIN_HEIGHT: f32 = 240.0;
const MAP_NODE_SIZE: f32 = 4.0;
const MAP_SELECTED_SIZE: f32 = 8.0;
const MAP_ANCHOR_SIZE: f32 = 10.0;
const MAP_HIT_RADIUS: f32 = 8.0;

pub(super) fn sample_map_view(
    items: Vec<SampleMapItem>,
    viewport: SampleMapViewport,
    name_filter: String,
) -> ui::View<GuiMessage> {
    let map = if items.is_empty() {
        ui::column([
            ui::text_line("No audio files in selected folder", 23.0).muted_text(),
            ui::spacer().fill_height(),
        ])
        .spacing(0.0)
        .fill()
    } else {
        ui::custom_widget_direct(SampleMapWidget::new(items, viewport))
            .id(widget_ids::SAMPLE_BROWSER_MAP_ID)
            .height(MAP_MIN_HEIGHT)
            .fill()
    };
    ui::stack([map, sample_map_search_overlay(name_filter)])
        .fill()
        .height(MAP_MIN_HEIGHT)
}

fn sample_map_search_overlay(name_filter: String) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::spacer().fill_width().height(26.0),
            ui::text_input(name_filter)
                .placeholder("Search")
                .clear_button(GuiMessage::FolderBrowser(
                    FolderBrowserMessage::NameFilterInput(TextInputMessage::Changed {
                        value: String::new(),
                    }),
                ))
                .id(widget_ids::SAMPLE_BROWSER_MAP_SEARCH_INPUT_ID)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::NameFilterInput(message))
                })
                .size(320.0, 24.0),
            ui::spacer().fill_width().height(26.0),
        ])
        .height(30.0)
        .padding_y(4.0)
        .fill_width(),
        ui::spacer().fill_height(),
    ])
    .fill()
}

#[derive(Clone, Debug)]
struct SampleMapWidget {
    common: WidgetCommon,
    gesture: CanvasGestureState,
    items: Vec<SampleMapItem>,
    viewport: SampleMapViewport,
    last_hit_file_id: Option<String>,
    last_primary_position: Option<Point>,
    last_pan_position: Option<Point>,
}

impl SampleMapWidget {
    fn new(items: Vec<SampleMapItem>, viewport: SampleMapViewport) -> Self {
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
            viewport,
            last_hit_file_id: None,
            last_primary_position: None,
            last_pan_position: None,
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

    fn message_for_swept_hit(
        &mut self,
        bounds: Rect,
        from: Point,
        to: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        if let Some(output) = self.message_for_hit(bounds, to, modifiers) {
            return Some(output);
        }
        let hit_file_id = self.hit_test_segment(bounds, from, to)?.file_id.clone();
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
            let center = item_center(bounds, item, self.viewport);
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

    fn hit_test_segment(&self, bounds: Rect, from: Point, to: Point) -> Option<&SampleMapItem> {
        let mut best: Option<(&SampleMapItem, f32)> = None;
        for item in &self.items {
            let center = item_center(bounds, item, self.viewport);
            let distance_sq = point_segment_distance_squared(center, from, to);
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
            } => {
                self.last_primary_position = Some(pointer.position);
                self.message_for_hit(bounds, pointer.position, modifiers)
            }
            CanvasGestureEvent::Drag {
                pointer,
                button: PointerButton::Primary,
                modifiers,
                ..
            } => {
                let previous = self
                    .last_primary_position
                    .replace(pointer.position)
                    .unwrap_or(pointer.position);
                self.message_for_swept_hit(bounds, previous, pointer.position, modifiers)
            }
            CanvasGestureEvent::Press {
                pointer,
                button: PointerButton::Secondary,
                ..
            } => {
                self.last_pan_position = Some(pointer.position);
                None
            }
            CanvasGestureEvent::Drag {
                pointer,
                button: PointerButton::Secondary,
                ..
            } => {
                let previous = self.last_pan_position.replace(pointer.position)?;
                Some(WidgetOutput::typed(GuiMessage::ChangeSampleMapViewport(
                    SampleMapViewportChange::Pan {
                        delta: Vector2::new(
                            (pointer.position.x - previous.x) / bounds.width().max(1.0),
                            (pointer.position.y - previous.y) / bounds.height().max(1.0),
                        ),
                    },
                )))
            }
            CanvasGestureEvent::Wheel { pointer, delta } => {
                let factor = if delta.y < 0.0 { 1.15 } else { 1.0 / 1.15 };
                Some(WidgetOutput::typed(GuiMessage::ChangeSampleMapViewport(
                    SampleMapViewportChange::Zoom {
                        anchor: pointer.normalized,
                        factor,
                    },
                )))
            }
            CanvasGestureEvent::DoubleClick { .. } => Some(WidgetOutput::typed(
                GuiMessage::ChangeSampleMapViewport(SampleMapViewportChange::Reset),
            )),
            CanvasGestureEvent::Release { .. } | CanvasGestureEvent::Drop { .. } => {
                self.last_hit_file_id = None;
                self.last_primary_position = None;
                self.last_pan_position = None;
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
            paint_item(primitives, self.common.id, bounds, item, self.viewport);
        }
    }
}

fn paint_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    item: &SampleMapItem,
    viewport: SampleMapViewport,
) {
    let center = item_center(bounds, item, viewport);
    if !paint_bounds(bounds).contains(center) {
        return;
    }
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

fn item_center(bounds: Rect, item: &SampleMapItem, viewport: SampleMapViewport) -> Point {
    Point::new(
        bounds.x_for_ratio_unclamped((item.x - viewport.center_x) * viewport.zoom + 0.5),
        bounds.y_for_ratio_unclamped((item.y - viewport.center_y) * viewport.zoom + 0.5),
    )
}

fn paint_bounds(bounds: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(
            bounds.min.x - MAP_ANCHOR_SIZE,
            bounds.min.y - MAP_ANCHOR_SIZE,
        ),
        Point::new(
            bounds.max.x + MAP_ANCHOR_SIZE,
            bounds.max.y + MAP_ANCHOR_SIZE,
        ),
    )
}

fn centered_rect(center: Point, side: f32) -> Rect {
    Rect::from_xy_size(center.x - side * 0.5, center.y - side * 0.5, side, side)
}

fn distance_squared(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn point_segment_distance_squared(point: Point, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return distance_squared(point, start);
    }
    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0);
    let closest = Point::new(start.x + dx * t, start.y + dy * t);
    distance_squared(point, closest)
}

#[cfg(test)]
mod tests {
    use radiant::widgets::WidgetInput;

    use super::*;

    #[test]
    fn primary_drag_auditions_node_crossed_between_pointer_samples() {
        let mut widget = SampleMapWidget::new(
            vec![SampleMapItem {
                file_id: String::from("/samples/clap.wav"),
                label: String::from("clap"),
                x: 0.5,
                y: 0.5,
                color: ui::Rgba8::new(255, 160, 80, 220),
                selected: false,
                similarity_anchor: false,
                missing: false,
            }],
            SampleMapViewport::default(),
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert!(
            widget
                .handle_input(bounds, WidgetInput::primary_press(Point::new(10.0, 50.0)))
                .is_none(),
            "press starts the drag away from the node"
        );
        let output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(190.0, 50.0)))
            .expect("swept drag should catch the crossed node");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::SelectSampleWithModifiers {
                path: String::from("/samples/clap.wav"),
                modifiers: PointerModifiers::default(),
            })
        );
    }

    #[test]
    fn point_segment_distance_detects_crossed_node() {
        assert_eq!(
            point_segment_distance_squared(
                Point::new(100.0, 50.0),
                Point::new(10.0, 50.0),
                Point::new(190.0, 50.0),
            ),
            0.0
        );
    }
}
