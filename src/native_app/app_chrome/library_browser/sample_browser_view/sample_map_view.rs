use radiant::{
    gui::types::{Point, Rect, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintPrimitive, push_fill_rect, push_fill_rect_batch, push_stroke_rect},
    theme::ThemeTokens,
    widgets::{
        CanvasGestureEvent, CanvasGestureState, PointerButton, PointerModifiers, TextInputMessage,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::collections::BTreeMap;

use crate::native_app::app::{GuiMessage, SampleMapViewport, SampleMapViewportChange};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::sample_map::{
    SampleMapItem, SampleMapStatus,
};
use crate::native_app::ui::ids as widget_ids;

const MAP_MIN_HEIGHT: f32 = 240.0;
const MAP_NODE_SIZE: f32 = 3.4;
const MAP_NODE_SIZE_DENSE: f32 = 2.2;
const MAP_NODE_SIZE_VERY_DENSE: f32 = 1.6;
const MAP_SELECTED_SIZE: f32 = 9.0;
const MAP_SELECTED_GLOW_SIZE: f32 = 17.0;
const MAP_ANCHOR_SIZE: f32 = 12.0;
const MAP_ANCHOR_GLOW_SIZE: f32 = 22.0;
const MAP_HIT_RADIUS: f32 = 8.0;
const MAP_DENSE_ITEM_COUNT: usize = 1_000;
const MAP_VERY_DENSE_ITEM_COUNT: usize = 4_000;

pub(super) fn sample_map_view(
    items: Vec<SampleMapItem>,
    viewport: SampleMapViewport,
    name_filter: String,
    status: SampleMapStatus,
    prep_running: bool,
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
    ui::stack([
        map,
        sample_map_search_overlay(name_filter),
        sample_map_status_overlay(status, prep_running),
    ])
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

fn sample_map_status_overlay(status: SampleMapStatus, prep_running: bool) -> ui::View<GuiMessage> {
    let Some(label) = status.label(prep_running) else {
        return ui::spacer().fill();
    };
    ui::column([
        ui::spacer().fill_height(),
        ui::row([
            ui::passive_badge(label)
                .style(ui::WidgetStyle::subtle(ui::WidgetTone::Warning))
                .height(20.0),
            ui::spacer().fill_width().height(20.0),
        ])
        .padding(8.0)
        .height(36.0)
        .fill_width(),
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
        paint_items(
            primitives,
            self.common.id,
            bounds,
            &self.items,
            self.viewport,
        );
    }
}

fn paint_items(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    items: &[SampleMapItem],
    viewport: SampleMapViewport,
) {
    let node_size = map_node_size(items.len());
    let mut batches = BTreeMap::<ColorKey, Vec<Rect>>::new();
    for item in items {
        queue_or_paint_item(
            primitives,
            widget_id,
            bounds,
            item,
            viewport,
            node_size,
            &mut batches,
        );
    }
    for (color, rects) in batches {
        push_fill_rect_batch(primitives, widget_id, rects, color.rgba());
    }
}

fn queue_or_paint_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    item: &SampleMapItem,
    viewport: SampleMapViewport,
    node_size: f32,
    batches: &mut BTreeMap<ColorKey, Vec<Rect>>,
) {
    let center = item_center(bounds, item, viewport);
    if !paint_bounds(bounds).contains(center) {
        return;
    }
    let color = sample_map_item_color(item);
    if item.selected || item.similarity_anchor {
        paint_highlight_item(primitives, widget_id, center, color, item.similarity_anchor);
        return;
    }
    batches
        .entry(ColorKey::from(color))
        .or_default()
        .push(centered_rect(center, node_size));
}

fn paint_highlight_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
    similarity_anchor: bool,
) {
    let size = if similarity_anchor {
        MAP_ANCHOR_SIZE
    } else {
        MAP_SELECTED_SIZE
    };
    let glow_size = if similarity_anchor {
        MAP_ANCHOR_GLOW_SIZE
    } else {
        MAP_SELECTED_GLOW_SIZE
    };
    push_fill_rect(
        primitives,
        widget_id,
        centered_rect(center, glow_size),
        color.with_alpha(42),
    );
    let rect = centered_rect(center, size);
    push_fill_rect(primitives, widget_id, rect, color);
    push_stroke_rect(
        primitives,
        widget_id,
        centered_rect(center, size + 4.0),
        ui::Rgba8::new(245, 245, 245, 220),
        1.0,
    );
}

fn sample_map_item_color(item: &SampleMapItem) -> ui::Rgba8 {
    if item.missing {
        ui::Rgba8::new(120, 120, 120, 180)
    } else {
        item.color
    }
}

fn map_node_size(item_count: usize) -> f32 {
    if item_count >= MAP_VERY_DENSE_ITEM_COUNT {
        MAP_NODE_SIZE_VERY_DENSE
    } else if item_count >= MAP_DENSE_ITEM_COUNT {
        MAP_NODE_SIZE_DENSE
    } else {
        MAP_NODE_SIZE
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ColorKey {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl ColorKey {
    fn rgba(self) -> ui::Rgba8 {
        ui::Rgba8::new(self.r, self.g, self.b, self.a)
    }
}

impl From<ui::Rgba8> for ColorKey {
    fn from(color: ui::Rgba8) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
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
    fn ordinary_sample_map_nodes_are_batched_by_color() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let widget = SampleMapWidget::new(
            vec![
                sample_map_item("/samples/kick.wav", 0.25, 0.25, color),
                sample_map_item("/samples/snare.wav", 0.50, 0.50, color),
                sample_map_item("/samples/hat.wav", 0.75, 0.75, color),
            ],
            SampleMapViewport::default(),
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let batches = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRectBatch(batch) => Some(batch),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].color, color);
        assert_eq!(batches[0].rects.len(), 3);
        assert!((batches[0].rects[0].width() - MAP_NODE_SIZE).abs() < 0.001);
    }

    #[test]
    fn selected_and_anchor_sample_map_nodes_paint_highlight_layers() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let mut selected = sample_map_item("/samples/kick.wav", 0.25, 0.5, color);
        selected.selected = true;
        let mut anchor = sample_map_item("/samples/snare.wav", 0.75, 0.5, color);
        anchor.similarity_anchor = true;
        let widget = SampleMapWidget::new(vec![selected, anchor], SampleMapViewport::default());
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let fills = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(fills.iter().any(|fill| fill.color == color.with_alpha(42)
            && fill.rect.width() == MAP_SELECTED_GLOW_SIZE));
        assert!(
            fills.iter().any(|fill| fill.color == color.with_alpha(42)
                && fill.rect.width() == MAP_ANCHOR_GLOW_SIZE)
        );
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokeRect(stroke)
                if stroke.color == ui::Rgba8::new(245, 245, 245, 220)
        )));
    }

    #[test]
    fn dense_sample_maps_use_smaller_node_sizes() {
        assert_eq!(map_node_size(10), MAP_NODE_SIZE);
        assert_eq!(map_node_size(MAP_DENSE_ITEM_COUNT), MAP_NODE_SIZE_DENSE);
        assert_eq!(
            map_node_size(MAP_VERY_DENSE_ITEM_COUNT),
            MAP_NODE_SIZE_VERY_DENSE
        );
    }

    #[test]
    fn primary_drag_auditions_node_crossed_between_pointer_samples() {
        let mut widget = SampleMapWidget::new(
            vec![sample_map_item(
                "/samples/clap.wav",
                0.5,
                0.5,
                ui::Rgba8::new(255, 160, 80, 220),
            )],
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

    fn sample_map_item(file_id: &str, x: f32, y: f32, color: ui::Rgba8) -> SampleMapItem {
        SampleMapItem {
            file_id: String::from(file_id),
            label: String::from(file_id),
            x,
            y,
            color,
            selected: false,
            similarity_anchor: false,
            missing: false,
        }
    }
}
