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
use std::collections::{BTreeMap, HashSet};

use crate::native_app::app::{
    GuiMessage, SampleMapAuditionDragState, SampleMapViewport, SampleMapViewportChange,
};
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
const MAP_GROUP_MIN_ITEMS: usize = 3;
const MAP_GROUP_REGION_PADDING: f32 = 18.0;
const MAP_DENSE_ITEM_COUNT: usize = 1_000;
const MAP_VERY_DENSE_ITEM_COUNT: usize = 4_000;
const MAP_CONTROL_ICON_ENABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(236, 239, 242, 255);
const MAP_CONTROL_ICON_ACTIVE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 255);
const MAP_CONTROL_ICON_TINTS: ui::SvgIconTintPalette = ui::SvgIconTintPalette::new(
    MAP_CONTROL_ICON_ENABLED_COLOR,
    MAP_CONTROL_ICON_ACTIVE_COLOR,
    MAP_CONTROL_ICON_ENABLED_COLOR,
);
const MAP_CONTROL_ZOOM_FACTOR: f32 = 1.35;
const MAP_CONTROL_ANCHOR: Vector2 = Vector2 { x: 0.5, y: 0.5 };

pub(super) fn sample_map_view(
    items: Vec<SampleMapItem>,
    viewport: SampleMapViewport,
    name_filter: String,
    status: SampleMapStatus,
    prep_running: bool,
    active_drag: Option<SampleMapAuditionDragState>,
) -> ui::View<GuiMessage> {
    let map = if items.is_empty() {
        ui::column([
            ui::text_line("No audio files in selected folder", 23.0).muted_text(),
            ui::spacer().fill_height(),
        ])
        .spacing(0.0)
        .fill()
    } else {
        ui::custom_widget_direct(SampleMapWidget::new(items, viewport, active_drag))
            .id(widget_ids::SAMPLE_BROWSER_MAP_ID)
            .height(MAP_MIN_HEIGHT)
            .fill()
    };
    ui::stack([
        map,
        sample_map_search_overlay(name_filter),
        sample_map_controls_overlay(),
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

fn sample_map_controls_overlay() -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill_width().height(36.0),
        ui::row([
            ui::spacer().fill_width().height(26.0),
            sample_map_control_button(
                sample_map_zoom_out_icon(),
                GuiMessage::ChangeSampleMapViewport(SampleMapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: 1.0 / MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom out"),
            sample_map_control_button(
                sample_map_zoom_in_icon(),
                GuiMessage::ChangeSampleMapViewport(SampleMapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom in"),
            sample_map_control_button(
                sample_map_focus_icon(),
                GuiMessage::FocusSelectedSampleMapNode,
            )
            .tooltip("Focus selected sample"),
            sample_map_control_button(
                sample_map_reset_icon(),
                GuiMessage::ChangeSampleMapViewport(SampleMapViewportChange::Reset),
            )
            .tooltip("Reset map view"),
        ])
        .spacing(4.0)
        .padding(8.0)
        .height(40.0)
        .fill_width(),
        ui::spacer().fill_height(),
    ])
    .fill()
}

fn sample_map_control_button(icon: ui::SvgIcon, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::icon_button(icon).message(message).size(24.0, 22.0)
}

fn sample_map_zoom_in_icon() -> ui::SvgIcon {
    MAP_ZOOM_IN_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn sample_map_zoom_out_icon() -> ui::SvgIcon {
    MAP_ZOOM_OUT_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn sample_map_focus_icon() -> ui::SvgIcon {
    MAP_FOCUS_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn sample_map_reset_icon() -> ui::SvgIcon {
    MAP_RESET_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
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
    active_drag: Option<SampleMapAuditionDragState>,
}

impl SampleMapWidget {
    fn new(
        items: Vec<SampleMapItem>,
        viewport: SampleMapViewport,
        active_drag: Option<SampleMapAuditionDragState>,
    ) -> Self {
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
            active_drag,
        }
    }

    fn hit_file_id(&self, bounds: Rect, point: Point) -> Option<String> {
        self.hit_test(bounds, point)
            .map(|item| item.file_id.clone())
    }

    fn begin_audition_drag_message(
        &mut self,
        bounds: Rect,
        point: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let hit_file_id = self.hit_file_id(bounds, point);
        self.last_hit_file_id = hit_file_id.clone();
        self.last_primary_position = Some(point);
        Some(WidgetOutput::typed(
            GuiMessage::BeginSampleMapAuditionDrag {
                path: hit_file_id,
                position: point,
                modifiers,
            },
        ))
    }

    fn update_audition_drag_message(
        &mut self,
        bounds: Rect,
        point: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let previous = self
            .active_drag
            .as_ref()
            .map(|drag| drag.last_position)
            .or(self.last_primary_position)
            .unwrap_or(point);
        let last_hit_file_id = self
            .active_drag
            .as_ref()
            .and_then(|drag| drag.last_hit_file_id.as_deref())
            .or(self.last_hit_file_id.as_deref());
        let mut hit_file_ids = self.hit_file_ids_between(bounds, previous, point);
        hit_file_ids.retain(|hit| Some(hit.as_str()) != last_hit_file_id);
        self.last_primary_position = Some(point);
        if hit_file_ids.is_empty() {
            return None;
        }
        if let Some(hit_file_id) = hit_file_ids.last() {
            self.last_hit_file_id = Some(hit_file_id.clone());
        }
        Some(WidgetOutput::typed(
            GuiMessage::UpdateSampleMapAuditionDrag {
                paths: hit_file_ids,
                position: point,
                modifiers,
            },
        ))
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

    fn hit_file_ids_between(&self, bounds: Rect, from: Point, to: Point) -> Vec<String> {
        let mut hits = Vec::new();
        for item in &self.items {
            let center = item_center(bounds, item, self.viewport);
            let distance_sq = point_segment_distance_squared(center, from, to);
            if distance_sq > MAP_HIT_RADIUS * MAP_HIT_RADIUS {
                continue;
            }
            hits.push(SampleMapSegmentHit {
                file_id: item.file_id.clone(),
                segment_t: point_segment_t(center, from, to),
                distance_sq,
            });
        }
        hits.sort_by(|a, b| {
            a.segment_t
                .total_cmp(&b.segment_t)
                .then_with(|| a.distance_sq.total_cmp(&b.distance_sq))
        });
        let mut seen = HashSet::new();
        hits.into_iter()
            .filter_map(|hit| seen.insert(hit.file_id.clone()).then_some(hit.file_id))
            .collect()
    }
}

#[derive(Clone, Debug)]
struct SampleMapSegmentHit {
    file_id: String,
    segment_t: f32,
    distance_sq: f32,
}

fn point_segment_t(point: Point, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return 1.0;
    }
    (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0)
}

fn point_segment_distance_squared(point: Point, start: Point, end: Point) -> f32 {
    let t = point_segment_t(point, start, end);
    if (t - 1.0).abs() <= f32::EPSILON && distance_squared(start, end) <= f32::EPSILON {
        return distance_squared(point, start);
    }
    let closest = Point::new(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    );
    distance_squared(point, closest)
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
            } => self.begin_audition_drag_message(bounds, pointer.position, modifiers),
            CanvasGestureEvent::Drag {
                pointer,
                button: PointerButton::Primary,
                modifiers,
                ..
            } => self.update_audition_drag_message(bounds, pointer.position, modifiers),
            CanvasGestureEvent::Hover(pointer) if self.active_drag.is_some() => self
                .update_audition_drag_message(
                    bounds,
                    pointer.position,
                    self.active_drag
                        .as_ref()
                        .map(|drag| drag.modifiers)
                        .unwrap_or_default(),
                ),
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
                Some(WidgetOutput::typed(GuiMessage::FinishSampleMapAuditionDrag))
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
        paint_group_regions(
            primitives,
            self.common.id,
            bounds,
            &self.items,
            self.viewport,
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

fn paint_group_regions(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    items: &[SampleMapItem],
    viewport: SampleMapViewport,
) {
    let mut regions = BTreeMap::<ColorHueKey, SampleMapGroupRegion>::new();
    for item in items {
        if item.missing {
            continue;
        }
        let center = item_center(bounds, item, viewport);
        if !paint_bounds(bounds).contains(center) {
            continue;
        }
        regions
            .entry(ColorHueKey::from(item.color))
            .or_insert_with(|| SampleMapGroupRegion::new(center, item.color))
            .include(center);
    }
    for region in regions.values() {
        if region.count < MAP_GROUP_MIN_ITEMS {
            continue;
        }
        let rect = region.rect().expanded(MAP_GROUP_REGION_PADDING);
        push_fill_rect(
            primitives,
            widget_id,
            rect,
            region
                .color
                .with_alpha(group_region_fill_alpha(region.count)),
        );
        push_stroke_rect(
            primitives,
            widget_id,
            rect,
            region
                .color
                .with_alpha(group_region_stroke_alpha(region.count)),
            1.0,
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct SampleMapGroupRegion {
    min: Point,
    max: Point,
    color: ui::Rgba8,
    count: usize,
}

impl SampleMapGroupRegion {
    fn new(center: Point, color: ui::Rgba8) -> Self {
        Self {
            min: center,
            max: center,
            color,
            count: 0,
        }
    }

    fn include(&mut self, center: Point) {
        self.min.x = self.min.x.min(center.x);
        self.min.y = self.min.y.min(center.y);
        self.max.x = self.max.x.max(center.x);
        self.max.y = self.max.y.max(center.y);
        self.count += 1;
    }

    fn rect(self) -> Rect {
        Rect::from_min_max(self.min, self.max)
    }
}

fn group_region_fill_alpha(count: usize) -> u8 {
    (12 + count.min(12) as u8 * 2).min(34)
}

fn group_region_stroke_alpha(count: usize) -> u8 {
    (24 + count.min(12) as u8 * 3).min(60)
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

trait SampleMapRectExt {
    fn expanded(self, padding: f32) -> Rect;
}

impl SampleMapRectExt for Rect {
    fn expanded(self, padding: f32) -> Rect {
        Rect::from_min_max(
            Point::new(self.min.x - padding, self.min.y - padding),
            Point::new(self.max.x + padding, self.max.y + padding),
        )
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ColorHueKey {
    r: u8,
    g: u8,
    b: u8,
}

impl From<ui::Rgba8> for ColorHueKey {
    fn from(color: ui::Rgba8) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

static MAP_ZOOM_IN_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="7" cy="7" r="4.4" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <path d="M7 4.8v4.4M4.8 7h4.4M10.4 10.4l3.1 3.1" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
</svg>"#,
);

static MAP_ZOOM_OUT_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <circle cx="7" cy="7" r="4.4" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <path d="M4.8 7h4.4M10.4 10.4l3.1 3.1" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
</svg>"#,
);

static MAP_FOCUS_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M8 2.4v2M8 11.6v2M2.4 8h2M11.6 8h2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  <circle cx="8" cy="8" r="2.7" fill="none" stroke="currentColor" stroke-width="1.5"/>
  <circle cx="8" cy="8" r="0.9" fill="currentColor"/>
</svg>"#,
);

static MAP_RESET_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4.3 5.2A4.7 4.7 0 1 1 3.8 10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
  <path d="M4.3 2.6v2.6H1.7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
);

fn distance_squared(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
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
            None,
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
    fn similarity_color_groups_paint_subtle_backdrop_regions() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let widget = SampleMapWidget::new(
            vec![
                sample_map_item("/samples/kick.wav", 0.25, 0.25, color.with_alpha(190)),
                sample_map_item("/samples/snare.wav", 0.50, 0.50, color.with_alpha(220)),
                sample_map_item("/samples/hat.wav", 0.75, 0.75, color.with_alpha(240)),
                sample_map_item(
                    "/samples/lone.wav",
                    0.90,
                    0.12,
                    ui::Rgba8::new(57, 187, 245, 220),
                ),
            ],
            SampleMapViewport::default(),
            None,
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == color.with_alpha(group_region_fill_alpha(3))
                    && fill.rect.width() > 100.0
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokeRect(stroke)
                if stroke.color == color.with_alpha(group_region_stroke_alpha(3))
        )));
        assert!(!primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == ui::Rgba8::new(57, 187, 245, group_region_fill_alpha(1))
        )));
    }

    #[test]
    fn selected_and_anchor_sample_map_nodes_paint_highlight_layers() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let mut selected = sample_map_item("/samples/kick.wav", 0.25, 0.5, color);
        selected.selected = true;
        let mut anchor = sample_map_item("/samples/snare.wav", 0.75, 0.5, color);
        anchor.similarity_anchor = true;
        let widget =
            SampleMapWidget::new(vec![selected, anchor], SampleMapViewport::default(), None);
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
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert_eq!(
            widget
                .handle_input(bounds, WidgetInput::primary_press(Point::new(10.0, 50.0)))
                .and_then(|output| output.typed_cloned::<GuiMessage>()),
            Some(GuiMessage::BeginSampleMapAuditionDrag {
                path: None,
                position: Point::new(10.0, 50.0),
                modifiers: PointerModifiers::default(),
            }),
            "press starts the drag even when it begins away from a node"
        );
        let output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(190.0, 50.0)))
            .expect("swept drag should catch the crossed node");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::UpdateSampleMapAuditionDrag {
                paths: vec![String::from("/samples/clap.wav")],
                position: Point::new(190.0, 50.0),
                modifiers: PointerModifiers::default(),
            })
        );
    }

    #[test]
    fn primary_drag_auditions_all_nodes_crossed_between_pointer_samples() {
        let mut widget = SampleMapWidget::new(
            vec![
                sample_map_item(
                    "/samples/kick.wav",
                    0.25,
                    0.5,
                    ui::Rgba8::new(255, 160, 80, 220),
                ),
                sample_map_item(
                    "/samples/snare.wav",
                    0.5,
                    0.5,
                    ui::Rgba8::new(57, 187, 245, 220),
                ),
                sample_map_item(
                    "/samples/hat.wav",
                    0.75,
                    0.5,
                    ui::Rgba8::new(125, 220, 140, 220),
                ),
            ],
            SampleMapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        widget
            .handle_input(bounds, WidgetInput::primary_press(Point::new(5.0, 50.0)))
            .expect("press starts audition drag");
        let output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(195.0, 50.0)))
            .expect("swept drag should catch every crossed node");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::UpdateSampleMapAuditionDrag {
                paths: vec![
                    String::from("/samples/kick.wav"),
                    String::from("/samples/snare.wav"),
                    String::from("/samples/hat.wav"),
                ],
                position: Point::new(195.0, 50.0),
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
