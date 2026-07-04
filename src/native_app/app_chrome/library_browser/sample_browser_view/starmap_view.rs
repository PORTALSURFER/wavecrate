use radiant::{
    gui::types::{Point, Rect, Vector2},
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        push_fill_polygon, push_fill_rect, push_fill_rect_batch, push_stroke_polyline,
        PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{
        CanvasGestureEvent, CanvasGestureState, PointerButton, PointerModifiers, TextInputMessage,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::native_app::app::{
    GuiMessage, StarmapAuditionDragState, StarmapViewport, StarmapViewportChange,
};
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextPointerAnchor, BrowserContextPointerTarget,
};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::starmap::{
    starmap_cluster_palette_color, StarmapItem, StarmapStatus,
};
use crate::native_app::starmap_audition_telemetry::{
    self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
};
use crate::native_app::ui::ids as widget_ids;
use wavecrate::sample_sources::config::SimilarityAspectSettings;
use wavecrate_analysis::aspects::SimilarityAspect;

use super::similarity_aspect_color;

const MAP_MIN_HEIGHT: f32 = 240.0;
const MAP_NODE_SIZE: f32 = 6.8;
const MAP_NODE_SIZE_DENSE: f32 = 4.4;
const MAP_NODE_SIZE_VERY_DENSE: f32 = 3.2;
const MAP_SELECTED_SIZE: f32 = 9.0;
const MAP_SELECTED_GLOW_SIZE: f32 = 17.0;
const MAP_SELECTED_OUTER_GLOW_SIZE: f32 = 25.0;
const MAP_ANCHOR_SIZE: f32 = 12.0;
const MAP_ANCHOR_GLOW_SIZE: f32 = 22.0;
const MAP_ACTIVE_AUDITION_SIZE: f32 = 11.0;
const MAP_ACTIVE_AUDITION_GLOW_SIZE: f32 = 24.0;
const MAP_COLD_AUDITION_SIZE_PAD: f32 = 3.0;
const MAP_HOVER_SIZE: f32 = 8.0;
const MAP_HOVER_GLOW_SIZE: f32 = 16.0;
const MAP_HIT_RADIUS: f32 = 8.0;
const MAP_HIT_GRID_CELL_SIZE: f32 = MAP_HIT_RADIUS * 2.0;
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
const MAP_LEGEND_SWATCH_SIZE: u8 = 7;

pub(super) fn starmap_view(
    items: impl Into<Arc<[StarmapItem]>>,
    viewport: StarmapViewport,
    name_filter: String,
    similarity_controls: &SimilarityAspectSettings,
    status: StarmapStatus,
    prep_running: bool,
    curation_mode_enabled: bool,
    active_drag: Option<StarmapAuditionDragState>,
) -> ui::View<GuiMessage> {
    let items = items.into();
    let map = if items.is_empty() {
        ui::column([
            ui::text_line(starmap_empty_message(curation_mode_enabled), 23.0).muted_text(),
            ui::spacer().fill_height(),
        ])
        .spacing(0.0)
        .fill()
    } else {
        ui::custom_widget_direct(StarmapWidget::new(items, viewport, active_drag))
            .id(widget_ids::SAMPLE_BROWSER_MAP_ID)
            .height(MAP_MIN_HEIGHT)
            .fill()
    };
    ui::stack([
        map,
        starmap_search_overlay(name_filter),
        starmap_controls_overlay(),
        starmap_legend_overlay(similarity_controls, status),
        starmap_status_overlay(status, prep_running),
    ])
    .fill()
    .height(MAP_MIN_HEIGHT)
}

fn starmap_empty_message(curation_mode_enabled: bool) -> &'static str {
    if curation_mode_enabled {
        "No files left to curate"
    } else {
        "No audio files in selected folder"
    }
}

fn starmap_search_overlay(name_filter: String) -> ui::View<GuiMessage> {
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

fn starmap_controls_overlay() -> ui::View<GuiMessage> {
    ui::column([
        ui::spacer().fill_width().height(36.0),
        ui::row([
            ui::spacer().fill_width().height(26.0),
            starmap_control_button(
                starmap_zoom_out_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: 1.0 / MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom out"),
            starmap_control_button(
                starmap_zoom_in_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Zoom {
                    anchor: MAP_CONTROL_ANCHOR,
                    factor: MAP_CONTROL_ZOOM_FACTOR,
                }),
            )
            .tooltip("Zoom in"),
            starmap_control_button(starmap_focus_icon(), GuiMessage::FocusSelectedStarmapNode)
                .tooltip("Focus selected sample"),
            starmap_control_button(
                starmap_reset_icon(),
                GuiMessage::ChangeStarmapViewport(StarmapViewportChange::Reset),
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

fn starmap_legend_overlay(
    controls: &SimilarityAspectSettings,
    status: StarmapStatus,
) -> ui::View<GuiMessage> {
    let entries = if status.clustered_count > 0 {
        starmap_cluster_legend_entries(status.cluster_color_count)
    } else {
        SimilarityAspect::ORDER
            .into_iter()
            .filter(|aspect| controls.aspect_enabled(*aspect))
            .map(starmap_aspect_legend_entry)
            .collect::<Vec<_>>()
    };
    if entries.is_empty() {
        return ui::spacer().fill();
    }
    ui::column([
        ui::spacer().fill_height(),
        ui::row([
            ui::spacer().fill_width().height(24.0),
            ui::row(entries)
                .spacing(7.0)
                .padding(6.0)
                .height(24.0)
                .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)),
        ])
        .padding(8.0)
        .height(40.0)
        .fill_width(),
    ])
    .fill()
}

fn starmap_cluster_legend_entries(cluster_color_count: usize) -> Vec<ui::View<GuiMessage>> {
    let swatch_count = cluster_color_count.clamp(1, 6);
    std::iter::once(starmap_text_legend_entry("Similarity clusters", 120.0))
        .chain((0..swatch_count).map(starmap_cluster_legend_swatch))
        .collect()
}

fn starmap_cluster_legend_swatch(index: usize) -> ui::View<GuiMessage> {
    ui::color_marker(Some(starmap_cluster_palette_color(index)))
        .side(MAP_LEGEND_SWATCH_SIZE)
        .inset(0)
        .view()
        .width(f32::from(MAP_LEGEND_SWATCH_SIZE) + 1.0)
        .height(16.0)
}

fn starmap_aspect_legend_entry(aspect: SimilarityAspect) -> ui::View<GuiMessage> {
    ui::row([
        ui::color_marker(Some(similarity_aspect_color(aspect)))
            .side(MAP_LEGEND_SWATCH_SIZE)
            .inset(0)
            .view()
            .width(f32::from(MAP_LEGEND_SWATCH_SIZE) + 1.0)
            .height(16.0),
        ui::text(starmap_aspect_label(aspect))
            .muted_text()
            .height(16.0)
            .width(starmap_aspect_label_width(aspect)),
    ])
    .spacing(3.0)
    .height(16.0)
}

fn starmap_text_legend_entry(label: &'static str, width: f32) -> ui::View<GuiMessage> {
    ui::text(label).muted_text().height(16.0).width(width)
}

fn starmap_aspect_label(aspect: SimilarityAspect) -> &'static str {
    match aspect {
        SimilarityAspect::Overall => "Overall",
        SimilarityAspect::Spectrum => "Spectrum",
        SimilarityAspect::Timbre => "Timbre",
        SimilarityAspect::Pitch => "Pitch",
        SimilarityAspect::Amplitude => "Amp",
    }
}

fn starmap_aspect_label_width(aspect: SimilarityAspect) -> f32 {
    match aspect {
        SimilarityAspect::Overall => 54.0,
        SimilarityAspect::Spectrum => 62.0,
        SimilarityAspect::Timbre => 48.0,
        SimilarityAspect::Pitch => 34.0,
        SimilarityAspect::Amplitude => 28.0,
    }
}

fn starmap_control_button(icon: ui::SvgIcon, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::icon_button(icon).message(message).size(24.0, 22.0)
}

fn starmap_zoom_in_icon() -> ui::SvgIcon {
    MAP_ZOOM_IN_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_zoom_out_icon() -> ui::SvgIcon {
    MAP_ZOOM_OUT_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_focus_icon() -> ui::SvgIcon {
    MAP_FOCUS_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_reset_icon() -> ui::SvgIcon {
    MAP_RESET_ICON.icon_for_state(MAP_CONTROL_ICON_TINTS, true, false)
}

fn starmap_status_overlay(status: StarmapStatus, prep_running: bool) -> ui::View<GuiMessage> {
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
struct StarmapWidget {
    common: WidgetCommon,
    gesture: CanvasGestureState,
    items: Arc<[StarmapItem]>,
    viewport: StarmapViewport,
    last_hit_file_id: Option<String>,
    last_hit_index: Option<usize>,
    last_primary_position: Option<Point>,
    last_pan_position: Option<Point>,
    active_drag: Option<StarmapAuditionDragState>,
    item_signature: u64,
    paint_signature: u64,
    hit_index: StarmapHitIndex,
    hit_scratch: Arc<Mutex<StarmapHitScratch>>,
    paint_cache: Arc<Mutex<StarmapPaintCache>>,
    hovered_file_id: Option<String>,
    hovered_item_index: Option<usize>,
    focused_item_index: Option<usize>,
}

impl StarmapWidget {
    fn new(
        items: impl Into<Arc<[StarmapItem]>>,
        viewport: StarmapViewport,
        active_drag: Option<StarmapAuditionDragState>,
    ) -> Self {
        let items = items.into();
        let common = WidgetCommon::new(
            widget_ids::SAMPLE_BROWSER_MAP_ID,
            WidgetSizing::new(
                Vector2::new(1.0, MAP_MIN_HEIGHT),
                Vector2::new(640.0, 360.0),
            ),
        )
        .with_pointer_focus()
        .without_default_chrome();
        let signatures = starmap_item_signatures(&items);
        let item_signature = signatures.hit;
        let paint_signature = signatures.paint;
        let focused_item_index = items.iter().position(|item| item.focused);
        Self {
            common,
            gesture: CanvasGestureState::new(),
            items,
            viewport,
            last_hit_file_id: None,
            last_hit_index: None,
            last_primary_position: None,
            last_pan_position: None,
            active_drag,
            item_signature,
            paint_signature,
            hit_index: StarmapHitIndex::default(),
            hit_scratch: Arc::new(Mutex::new(StarmapHitScratch::default())),
            paint_cache: Arc::new(Mutex::new(StarmapPaintCache::default())),
            hovered_file_id: None,
            hovered_item_index: None,
            focused_item_index,
        }
    }

    fn begin_audition_drag_message(
        &mut self,
        bounds: Rect,
        point: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
        let hit_started_at = starmap_telemetry::stage_timer();
        let hit_index = self.hit_item_index(bounds, point);
        let hit_elapsed = starmap_telemetry::elapsed_since(hit_started_at);
        if let Some(elapsed) = hit_elapsed {
            starmap_telemetry::record_duration(StarmapAuditionDuration::WidgetHitTest, elapsed);
        }
        let hit_file_id = hit_index.map(|index| self.items[index].file_id.clone());
        starmap_telemetry::record_event(
            Some(if hit_file_id.is_some() {
                StarmapAuditionCounter::WidgetPointHit
            } else {
                StarmapAuditionCounter::WidgetPointMiss
            }),
            "widget.point_hit_test",
            if hit_file_id.is_some() { "hit" } else { "miss" },
            hit_file_id.as_deref(),
            usize::from(hit_file_id.is_some()),
            0,
            false,
            hit_elapsed,
        );
        self.last_hit_file_id = hit_file_id.clone();
        self.last_hit_index = hit_index;
        self.last_primary_position = Some(point);
        Some(WidgetOutput::typed(GuiMessage::BeginStarmapAuditionDrag {
            path: hit_file_id,
            position: point,
            modifiers,
        }))
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
            .or(self.last_hit_file_id.as_deref())
            .map(str::to_owned);
        self.last_primary_position = Some(point);
        let hit_started_at = starmap_telemetry::stage_timer();
        let hit = self.hit_between(bounds, previous, point);
        let hit_elapsed = starmap_telemetry::elapsed_since(hit_started_at);
        if let Some(elapsed) = hit_elapsed {
            starmap_telemetry::record_duration(StarmapAuditionDuration::WidgetHitTest, elapsed);
        }
        let Some(hit) = hit else {
            starmap_telemetry::record_event(
                Some(StarmapAuditionCounter::WidgetSegmentMiss),
                "widget.segment_hit_test",
                "miss",
                None,
                0,
                0,
                last_hit_file_id.is_some(),
                hit_elapsed,
            );
            return None;
        };
        let hit_file_id = hit.file_id;
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::WidgetSegmentHit),
            "widget.segment_hit_test",
            "hit",
            Some(hit_file_id.as_str()),
            1,
            0,
            last_hit_file_id.is_some(),
            hit_elapsed,
        );
        if Some(&hit_file_id) == last_hit_file_id.as_ref() {
            return None;
        }
        self.last_hit_file_id = Some(hit_file_id.clone());
        self.last_hit_index = Some(hit.item_index);
        starmap_telemetry::record_event(
            None,
            "widget.drag_update",
            "hit_changed",
            Some(hit_file_id.as_str()),
            1,
            0,
            true,
            None,
        );
        Some(WidgetOutput::typed(GuiMessage::UpdateStarmapAuditionDrag {
            paths: vec![hit_file_id],
            position: point,
            modifiers,
        }))
    }

    fn hit_item_index(&mut self, bounds: Rect, point: Point) -> Option<usize> {
        self.ensure_hit_index(bounds);
        let mut best: Option<(usize, f32)> = None;
        let mut hit_scratch = lock_starmap_mutex(&self.hit_scratch);
        let candidates = self.hit_index.collect_item_indices_near_point(
            point,
            self.items.len(),
            &mut hit_scratch,
        );
        for &index in candidates {
            let item = &self.items[index];
            let center = item_center(bounds, item, self.viewport);
            let distance_sq = distance_squared(center, point);
            if distance_sq > MAP_HIT_RADIUS * MAP_HIT_RADIUS {
                continue;
            }
            if best.is_none_or(|(_, best_distance)| distance_sq < best_distance) {
                best = Some((index, distance_sq));
            }
        }
        best.map(|(index, _)| index)
    }

    fn hit_between(&mut self, bounds: Rect, from: Point, to: Point) -> Option<StarmapSegmentHit> {
        self.ensure_hit_index(bounds);
        let mut best: Option<StarmapSegmentHit> = None;
        let mut hit_scratch = lock_starmap_mutex(&self.hit_scratch);
        let candidates = self.hit_index.collect_item_indices_near_segment(
            from,
            to,
            self.items.len(),
            &mut hit_scratch,
        );
        for &index in candidates {
            let item = &self.items[index];
            let center = item_center(bounds, item, self.viewport);
            let distance_sq = point_segment_distance_squared(center, from, to);
            if distance_sq > MAP_HIT_RADIUS * MAP_HIT_RADIUS {
                continue;
            }
            let hit = StarmapSegmentHit {
                item_index: index,
                file_id: item.file_id.clone(),
                segment_t: point_segment_t(center, from, to),
                distance_sq,
            };
            if best.as_ref().is_none_or(|best| {
                hit.segment_t
                    .total_cmp(&best.segment_t)
                    .then_with(|| best.distance_sq.total_cmp(&hit.distance_sq))
                    .is_gt()
            }) {
                best = Some(hit);
            }
        }
        best
    }

    fn ensure_hit_index(&mut self, bounds: Rect) {
        if self
            .hit_index
            .matches(bounds, self.viewport, self.item_signature)
        {
            return;
        }
        self.hit_index =
            StarmapHitIndex::build(bounds, self.viewport, self.item_signature, &self.items);
    }

    fn set_hovered_file_at(&mut self, bounds: Rect, point: Point) {
        let index = self.hit_item_index(bounds, point);
        self.hovered_item_index = index;
        self.hovered_file_id = index.map(|index| self.items[index].file_id.clone());
    }

    fn remember_hovered_context_menu_anchor(&self, point: Point) -> Option<WidgetOutput> {
        self.hovered_file_id.as_ref().map(|file_id| {
            WidgetOutput::typed(GuiMessage::RememberBrowserContextMenuPointerAnchor(
                BrowserContextPointerAnchor {
                    target: BrowserContextPointerTarget::Sample(file_id.clone()),
                    position: point,
                },
            ))
        })
    }

    fn hovered_item(&self) -> Option<&StarmapItem> {
        let hovered_file_id = self.hovered_file_id.as_deref()?;
        self.item_for_cached_index(self.hovered_item_index, hovered_file_id)
            .or_else(|| {
                self.items
                    .iter()
                    .find(|item| item.file_id.as_str() == hovered_file_id)
            })
    }

    fn active_drag_item(&self) -> Option<&StarmapItem> {
        let active_file_id = self.active_drag.as_ref()?.last_hit_file_id.as_deref()?;
        self.item_for_cached_index(self.last_hit_index, active_file_id)
            .or_else(|| {
                self.items
                    .iter()
                    .find(|item| item.file_id.as_str() == active_file_id)
            })
    }

    fn focused_item(&self) -> Option<&StarmapItem> {
        self.items.get(self.focused_item_index?)
    }

    fn item_for_cached_index(
        &self,
        index: Option<usize>,
        expected_file_id: &str,
    ) -> Option<&StarmapItem> {
        let item = self.items.get(index?)?;
        (item.file_id == expected_file_id).then_some(item)
    }
}

#[derive(Clone, Debug)]
struct StarmapSegmentHit {
    item_index: usize,
    file_id: String,
    segment_t: f32,
    distance_sq: f32,
}

#[derive(Clone, Debug, Default)]
struct StarmapHitIndex {
    bounds: Option<Rect>,
    viewport: Option<StarmapViewport>,
    item_signature: u64,
    cells: Arc<HashMap<StarmapGridCell, Vec<usize>>>,
}

impl StarmapHitIndex {
    fn build(
        bounds: Rect,
        viewport: StarmapViewport,
        item_signature: u64,
        items: &[StarmapItem],
    ) -> Self {
        let mut cells = HashMap::<StarmapGridCell, Vec<usize>>::new();
        let indexed_bounds = paint_bounds(bounds).expanded(MAP_HIT_RADIUS);
        for (index, item) in items.iter().enumerate() {
            let center = item_center(bounds, item, viewport);
            if !indexed_bounds.contains(center) {
                continue;
            }
            cells
                .entry(StarmapGridCell::from_point(center))
                .or_default()
                .push(index);
        }
        Self {
            bounds: Some(bounds),
            viewport: Some(viewport),
            item_signature,
            cells: Arc::new(cells),
        }
    }

    fn matches(&self, bounds: Rect, viewport: StarmapViewport, item_signature: u64) -> bool {
        self.bounds == Some(bounds)
            && self.viewport == Some(viewport)
            && self.item_signature == item_signature
    }

    fn matches_current(&self, viewport: StarmapViewport, item_signature: u64) -> bool {
        self.bounds.is_some()
            && self.viewport == Some(viewport)
            && self.item_signature == item_signature
    }

    fn collect_item_indices_near_point<'a>(
        &'a self,
        point: Point,
        item_count: usize,
        scratch: &'a mut StarmapHitScratch,
    ) -> &'a [usize] {
        self.collect_item_indices_for_rect(
            centered_rect(point, MAP_HIT_RADIUS * 2.0),
            item_count,
            scratch,
        )
    }

    fn collect_item_indices_near_segment<'a>(
        &'a self,
        from: Point,
        to: Point,
        item_count: usize,
        scratch: &'a mut StarmapHitScratch,
    ) -> &'a [usize] {
        self.collect_item_indices_for_rect(
            segment_bounds(from, to).expanded(MAP_HIT_RADIUS),
            item_count,
            scratch,
        )
    }

    fn collect_item_indices_for_rect<'a>(
        &'a self,
        rect: Rect,
        item_count: usize,
        scratch: &'a mut StarmapHitScratch,
    ) -> &'a [usize] {
        scratch.begin(item_count);
        let min = StarmapGridCell::from_point(rect.min);
        let max = StarmapGridCell::from_point(rect.max);
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let Some(cell_indices) = self.cells.get(&StarmapGridCell { x, y }) else {
                    continue;
                };
                for &index in cell_indices {
                    if scratch.mark_seen(index) {
                        scratch.indices.push(index);
                    }
                }
            }
        }
        scratch.indices.as_slice()
    }

    #[cfg(test)]
    fn item_indices_near_segment(&self, from: Point, to: Point, item_count: usize) -> Vec<usize> {
        let mut scratch = StarmapHitScratch::default();
        self.collect_item_indices_near_segment(from, to, item_count, &mut scratch)
            .to_vec()
    }
}

#[derive(Clone, Debug, Default)]
struct StarmapHitScratch {
    generation: u32,
    seen_generation: Vec<u32>,
    indices: Vec<usize>,
}

impl StarmapHitScratch {
    fn begin(&mut self, item_count: usize) {
        self.indices.clear();
        if self.seen_generation.len() < item_count {
            self.seen_generation.resize(item_count, 0);
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.seen_generation.fill(0);
            self.generation = 1;
        }
    }

    fn mark_seen(&mut self, index: usize) -> bool {
        let Some(seen_generation) = self.seen_generation.get_mut(index) else {
            return false;
        };
        if *seen_generation == self.generation {
            return false;
        }
        *seen_generation = self.generation;
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct StarmapGridCell {
    x: i32,
    y: i32,
}

impl StarmapGridCell {
    fn from_point(point: Point) -> Self {
        Self {
            x: grid_coordinate(point.x),
            y: grid_coordinate(point.y),
        }
    }
}

fn grid_coordinate(value: f32) -> i32 {
    (value / MAP_HIT_GRID_CELL_SIZE).floor() as i32
}

fn segment_bounds(from: Point, to: Point) -> Rect {
    Rect::from_min_max(
        Point::new(from.x.min(to.x), from.y.min(to.y)),
        Point::new(from.x.max(to.x), from.y.max(to.y)),
    )
}

#[cfg(test)]
fn starmap_items_signature(items: &[StarmapItem]) -> u64 {
    starmap_item_signatures(items).hit
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StarmapItemSignatures {
    hit: u64,
    paint: u64,
}

fn starmap_item_signatures(items: &[StarmapItem]) -> StarmapItemSignatures {
    let mut hit_hasher = DefaultHasher::new();
    let mut paint_hasher = DefaultHasher::new();
    items.len().hash(&mut hit_hasher);
    items.len().hash(&mut paint_hasher);
    for item in items {
        item.file_id.hash(&mut hit_hasher);
        item.x.to_bits().hash(&mut hit_hasher);
        item.y.to_bits().hash(&mut hit_hasher);

        item.file_id.hash(&mut paint_hasher);
        item.x.to_bits().hash(&mut paint_hasher);
        item.y.to_bits().hash(&mut paint_hasher);
        item.color.r.hash(&mut paint_hasher);
        item.color.g.hash(&mut paint_hasher);
        item.color.b.hash(&mut paint_hasher);
        item.color.a.hash(&mut paint_hasher);
        item.selected.hash(&mut paint_hasher);
        item.copy_flash.hash(&mut paint_hasher);
        item.similarity_anchor.hash(&mut paint_hasher);
        item.instant_audition_ready.hash(&mut paint_hasher);
        item.missing.hash(&mut paint_hasher);
    }
    StarmapItemSignatures {
        hit: hit_hasher.finish(),
        paint: paint_hasher.finish(),
    }
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

impl Widget for StarmapWidget {
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
            CanvasGestureEvent::Hover(pointer) => {
                self.set_hovered_file_at(bounds, pointer.position);
                self.remember_hovered_context_menu_anchor(pointer.position)
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
                Some(WidgetOutput::typed(GuiMessage::ChangeStarmapViewport(
                    StarmapViewportChange::Pan {
                        delta: Vector2::new(
                            (pointer.position.x - previous.x) / bounds.width().max(1.0),
                            (pointer.position.y - previous.y) / bounds.height().max(1.0),
                        ),
                    },
                )))
            }
            CanvasGestureEvent::Wheel { pointer, delta } => {
                let factor = if delta.y < 0.0 { 1.15 } else { 1.0 / 1.15 };
                Some(WidgetOutput::typed(GuiMessage::ChangeStarmapViewport(
                    StarmapViewportChange::Zoom {
                        anchor: pointer.normalized,
                        factor,
                    },
                )))
            }
            CanvasGestureEvent::DoubleClick {
                pointer,
                button: PointerButton::Primary,
                modifiers,
            } => self.begin_audition_drag_message(bounds, pointer.position, modifiers),
            CanvasGestureEvent::DoubleClick { .. } => None,
            CanvasGestureEvent::Release {
                button: PointerButton::Primary,
                ..
            }
            | CanvasGestureEvent::Drop {
                button: PointerButton::Primary,
                ..
            } => {
                self.last_hit_file_id = None;
                self.last_primary_position = None;
                self.last_pan_position = None;
                Some(WidgetOutput::typed(GuiMessage::FinishStarmapAuditionDrag))
            }
            CanvasGestureEvent::Release {
                button: PointerButton::Secondary,
                ..
            }
            | CanvasGestureEvent::Drop {
                button: PointerButton::Secondary,
                ..
            } => {
                self.last_pan_position = None;
                None
            }
            CanvasGestureEvent::Release { .. } | CanvasGestureEvent::Drop { .. } => None,
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.gesture = previous.gesture.clone();
        self.last_hit_file_id = previous.last_hit_file_id.clone();
        self.last_hit_index = previous.last_hit_index;
        self.last_primary_position = previous.last_primary_position;
        self.last_pan_position = previous.last_pan_position;
        self.hovered_file_id = previous.hovered_file_id.clone();
        self.hovered_item_index = previous.hovered_item_index;
        self.hit_scratch = previous.hit_scratch.clone();
        self.paint_cache = previous.paint_cache.clone();
        if previous
            .hit_index
            .matches_current(self.viewport, self.item_signature)
        {
            self.hit_index = previous.hit_index.clone();
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn accepts_wheel_input(&self) -> bool {
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
        append_cached_items_paint(
            primitives,
            self.common.id,
            bounds,
            &self.items,
            self.viewport,
            self.paint_signature,
            &self.paint_cache,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        if self.active_drag.is_some() {
            if let Some(item) = self.active_drag_item() {
                let center = item_center(bounds, item, self.viewport);
                if paint_bounds(bounds).contains(center) {
                    paint_active_audition_item(
                        primitives,
                        self.common.id,
                        center,
                        starmap_item_color(item),
                    );
                }
            }
            return;
        }
        let Some(item) = self.hovered_item() else {
            let Some(item) = self.focused_item() else {
                return;
            };
            let center = item_center(bounds, item, self.viewport);
            if !paint_bounds(bounds).contains(center) {
                return;
            }
            paint_focused_item(primitives, self.common.id, center, starmap_item_color(item));
            return;
        };
        let center = item_center(bounds, item, self.viewport);
        if !paint_bounds(bounds).contains(center) {
            return;
        }
        paint_hover_item(primitives, self.common.id, center, starmap_item_color(item));
    }
}

fn append_cached_items_paint(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    items: &[StarmapItem],
    viewport: StarmapViewport,
    paint_signature: u64,
    cache: &Mutex<StarmapPaintCache>,
) {
    let started_at = starmap_telemetry::stage_timer();
    if let Some(cached) = cached_item_paint(cache, bounds, viewport, paint_signature) {
        starmap_telemetry::record_event(
            Some(StarmapAuditionCounter::WidgetPaintCacheHit),
            "widget.paint_cache",
            "hit",
            None,
            items.len(),
            cached.len(),
            false,
            starmap_telemetry::elapsed_since(started_at),
        );
        primitives.extend(cached.iter().cloned());
        return;
    }

    let mut item_primitives = Vec::new();
    paint_items(&mut item_primitives, widget_id, bounds, items, viewport);
    let elapsed = starmap_telemetry::elapsed_since(started_at);
    if let Some(elapsed) = elapsed {
        starmap_telemetry::record_duration(StarmapAuditionDuration::WidgetPaintBuild, elapsed);
    }
    let item_primitives = Arc::<[PaintPrimitive]>::from(item_primitives);
    store_cached_item_paint(
        cache,
        bounds,
        viewport,
        paint_signature,
        item_primitives.clone(),
    );
    starmap_telemetry::record_event(
        Some(StarmapAuditionCounter::WidgetPaintCacheMiss),
        "widget.paint_cache",
        "miss",
        None,
        items.len(),
        item_primitives.len(),
        false,
        elapsed,
    );
    primitives.extend(item_primitives.iter().cloned());
}

fn cached_item_paint(
    cache: &Mutex<StarmapPaintCache>,
    bounds: Rect,
    viewport: StarmapViewport,
    paint_signature: u64,
) -> Option<Arc<[PaintPrimitive]>> {
    lock_starmap_mutex(cache)
        .entry
        .as_ref()
        .filter(|entry| entry.matches(bounds, viewport, paint_signature))
        .map(|entry| entry.primitives.clone())
}

fn store_cached_item_paint(
    cache: &Mutex<StarmapPaintCache>,
    bounds: Rect,
    viewport: StarmapViewport,
    paint_signature: u64,
    primitives: Arc<[PaintPrimitive]>,
) {
    lock_starmap_mutex(cache).entry = Some(StarmapPaintCacheEntry {
        bounds,
        viewport,
        paint_signature,
        primitives,
    });
}

fn lock_starmap_mutex<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Clone, Debug, Default)]
struct StarmapPaintCache {
    entry: Option<StarmapPaintCacheEntry>,
}

#[derive(Clone, Debug)]
struct StarmapPaintCacheEntry {
    bounds: Rect,
    viewport: StarmapViewport,
    paint_signature: u64,
    primitives: Arc<[PaintPrimitive]>,
}

impl StarmapPaintCacheEntry {
    fn matches(&self, bounds: Rect, viewport: StarmapViewport, paint_signature: u64) -> bool {
        self.bounds == bounds
            && self.viewport == viewport
            && self.paint_signature == paint_signature
    }
}

fn paint_items(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    bounds: Rect,
    items: &[StarmapItem],
    viewport: StarmapViewport,
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
    item: &StarmapItem,
    viewport: StarmapViewport,
    node_size: f32,
    batches: &mut BTreeMap<ColorKey, Vec<Rect>>,
) {
    let center = item_center(bounds, item, viewport);
    if !paint_bounds(bounds).contains(center) {
        return;
    }
    let color = starmap_item_color(item);
    if item.copy_flash {
        paint_copy_flash_item(primitives, widget_id, center, color);
    }
    if item.selected {
        paint_selected_item(primitives, widget_id, center, color);
        return;
    }
    if item.similarity_anchor {
        paint_similarity_anchor_item(primitives, widget_id, center, color);
        return;
    }
    if !item.instant_audition_ready {
        paint_cold_audition_item(primitives, widget_id, center, node_size, color);
        return;
    }
    batches
        .entry(ColorKey::from(color))
        .or_default()
        .push(centered_rect(center, node_size));
}

fn paint_cold_audition_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    node_size: f32,
    color: ui::Rgba8,
) {
    let side = node_size + MAP_COLD_AUDITION_SIZE_PAD;
    paint_diamond(primitives, widget_id, center, side, color.with_alpha(46));
    stroke_diamond(
        primitives,
        widget_id,
        center,
        side,
        ui::Rgba8::new(232, 236, 238, 165),
        1.0,
    );
}

fn paint_copy_flash_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_GLOW_SIZE + 6.0,
        color.with_alpha(78),
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_SIZE + 7.0,
        ui::Rgba8::new(245, 245, 245, 235),
        1.25,
    );
}

fn paint_selected_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_OUTER_GLOW_SIZE,
        color.with_alpha(64),
    );
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_GLOW_SIZE,
        color.with_alpha(118),
    );
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_SIZE + 2.0,
        color.with_alpha(255),
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_SIZE + 6.0,
        ui::Rgba8::new(255, 252, 229, 245),
        1.35,
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_SELECTED_SIZE + 1.5,
        ui::Rgba8::new(255, 255, 255, 210),
        0.85,
    );
}

fn paint_similarity_anchor_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_ANCHOR_GLOW_SIZE,
        color.with_alpha(42),
    );
    paint_diamond(primitives, widget_id, center, MAP_ANCHOR_SIZE, color);
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_ANCHOR_SIZE + 4.0,
        ui::Rgba8::new(245, 245, 245, 220),
        1.0,
    );
}

fn paint_hover_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_HOVER_GLOW_SIZE,
        color.with_alpha(50),
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_HOVER_SIZE,
        ui::Rgba8::new(248, 248, 248, 230),
        1.0,
    );
}

fn paint_focused_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_HOVER_GLOW_SIZE,
        color.with_alpha(42),
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_HOVER_SIZE,
        ui::Rgba8::new(248, 248, 248, 190),
        1.0,
    );
}

fn paint_active_audition_item(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    color: ui::Rgba8,
) {
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_ACTIVE_AUDITION_GLOW_SIZE,
        color.with_alpha(70),
    );
    paint_diamond(
        primitives,
        widget_id,
        center,
        MAP_ACTIVE_AUDITION_SIZE,
        color.with_alpha(245),
    );
    stroke_diamond(
        primitives,
        widget_id,
        center,
        MAP_ACTIVE_AUDITION_SIZE + 5.0,
        ui::Rgba8::new(255, 250, 224, 245),
        1.25,
    );
}

fn paint_diamond(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    side: f32,
    color: ui::Rgba8,
) {
    push_fill_polygon(primitives, widget_id, diamond_points(center, side), color);
}

fn stroke_diamond(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    center: Point,
    side: f32,
    color: ui::Rgba8,
    width: f32,
) {
    push_stroke_polyline(
        primitives,
        widget_id,
        diamond_outline_points(center, side),
        color,
        width,
    );
}

fn starmap_item_color(item: &StarmapItem) -> ui::Rgba8 {
    if item.missing {
        ui::Rgba8::new(120, 120, 120, 180)
    } else if !item.instant_audition_ready {
        item.color.with_alpha(item.color.a.min(150))
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

fn item_center(bounds: Rect, item: &StarmapItem, viewport: StarmapViewport) -> Point {
    Point::new(
        bounds.x_for_ratio_unclamped((item.x - viewport.center_x) * viewport.zoom + 0.5),
        bounds.y_for_ratio_unclamped((item.y - viewport.center_y) * viewport.zoom + 0.5),
    )
}

fn paint_bounds(bounds: Rect) -> Rect {
    let margin = MAP_SELECTED_OUTER_GLOW_SIZE * 0.5;
    Rect::from_min_max(
        Point::new(bounds.min.x - margin, bounds.min.y - margin),
        Point::new(bounds.max.x + margin, bounds.max.y + margin),
    )
}

fn centered_rect(center: Point, side: f32) -> Rect {
    Rect::from_xy_size(center.x - side * 0.5, center.y - side * 0.5, side, side)
}

fn diamond_points(center: Point, side: f32) -> [Point; 4] {
    let radius = side * 0.5;
    [
        Point::new(center.x, center.y - radius),
        Point::new(center.x + radius, center.y),
        Point::new(center.x, center.y + radius),
        Point::new(center.x - radius, center.y),
    ]
}

fn diamond_outline_points(center: Point, side: f32) -> [Point; 5] {
    let [top, right, bottom, left] = diamond_points(center, side);
    [top, right, bottom, left, top]
}

trait StarmapRectExt {
    fn expanded(self, padding: f32) -> Rect;
}

impl StarmapRectExt for Rect {
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
    fn ordinary_starmap_nodes_are_batched_by_color() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let widget = StarmapWidget::new(
            vec![
                starmap_item("/samples/kick.wav", 0.25, 0.25, color),
                starmap_item("/samples/snare.wav", 0.50, 0.50, color),
                starmap_item("/samples/hat.wav", 0.75, 0.75, color),
            ],
            StarmapViewport::default(),
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
    fn similarity_color_groups_do_not_paint_backdrop_regions() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let items = (0..12)
            .map(|index| {
                starmap_item(
                    &format!("/samples/group-{index}.wav"),
                    0.25 + index as f32 * 0.04,
                    0.25 + index as f32 * 0.04,
                    color.with_alpha(190 + index.min(4) as u8 * 10),
                )
            })
            .chain(std::iter::once(starmap_item(
                "/samples/lone.wav",
                0.90,
                0.12,
                ui::Rgba8::new(57, 187, 245, 220),
            )))
            .collect::<Vec<_>>();
        let widget = StarmapWidget::new(items, StarmapViewport::default(), None);
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(!primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color.a < 100
                    && (fill.rect.width() > MAP_ACTIVE_AUDITION_GLOW_SIZE
                        || fill.rect.height() > MAP_ACTIVE_AUDITION_GLOW_SIZE)
        )));
        assert!(!primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokeRect(stroke)
                if stroke.color.a < 100
                    && (stroke.rect.width() > MAP_ACTIVE_AUDITION_GLOW_SIZE
                        || stroke.rect.height() > MAP_ACTIVE_AUDITION_GLOW_SIZE)
        )));
    }

    #[test]
    fn same_color_runs_still_paint_individual_nodes() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let widget = StarmapWidget::new(
            vec![
                starmap_item("/samples/kick.wav", 0.25, 0.25, color.with_alpha(190)),
                starmap_item("/samples/snare.wav", 0.50, 0.50, color.with_alpha(220)),
                starmap_item("/samples/hat.wav", 0.75, 0.75, color.with_alpha(240)),
            ],
            StarmapViewport::default(),
            None,
        );
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let node_count = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRectBatch(batch)
                    if (batch.color.r, batch.color.g, batch.color.b)
                        == (color.r, color.g, color.b) =>
                {
                    Some(batch.rects.len())
                }
                _ => None,
            })
            .sum::<usize>();
        assert_eq!(node_count, 3);
    }

    #[test]
    fn cold_audition_nodes_paint_as_hollow_markers() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let mut cold = starmap_item("/samples/long.wav", 0.50, 0.50, color);
        cold.instant_audition_ready = false;
        let widget = StarmapWidget::new(vec![cold], StarmapViewport::default(), None);
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillPolygon(fill) if fill.color.a < 80
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(232, 236, 238, 165)
        )));
        assert!(primitives.iter().all(|primitive| !matches!(
            primitive,
            PaintPrimitive::FillRectBatch(batch) if batch.color == color
        )));
    }

    #[test]
    fn copied_starmap_nodes_paint_confirmation_glow() {
        let color = ui::Rgba8::new(255, 160, 80, 220);
        let mut copied = starmap_item("/samples/copied.wav", 0.50, 0.50, color);
        copied.copy_flash = true;
        let widget = StarmapWidget::new(vec![copied], StarmapViewport::default(), None);
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            Rect::from_size(200.0, 100.0),
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillPolygon(fill)
                if fill.color == color.with_alpha(78)
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(245, 245, 245, 235)
        )));
    }

    #[test]
    fn selected_starmap_nodes_paint_stronger_than_similarity_anchor() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let mut selected = starmap_item("/samples/kick.wav", 0.25, 0.5, color);
        selected.selected = true;
        let mut anchor = starmap_item("/samples/snare.wav", 0.75, 0.5, color);
        anchor.similarity_anchor = true;
        let widget = StarmapWidget::new(vec![selected, anchor], StarmapViewport::default(), None);
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
                PaintPrimitive::FillPolygon(fill) => Some(fill),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(fills.iter().any(|fill| fill.color == color.with_alpha(64)
            && fill.points.len() == 4
            && fill.points[0] == Point::new(50.0, 50.0 - MAP_SELECTED_OUTER_GLOW_SIZE * 0.5)));
        assert!(fills.iter().any(|fill| fill.color == color.with_alpha(118)
            && fill.points.len() == 4
            && fill.points[0] == Point::new(50.0, 50.0 - MAP_SELECTED_GLOW_SIZE * 0.5)));
        assert!(fills.iter().any(|fill| fill.color == color.with_alpha(255)
            && fill.points.len() == 4
            && fill.points[0] == Point::new(50.0, 50.0 - (MAP_SELECTED_SIZE + 2.0) * 0.5)));
        assert!(fills.iter().any(|fill| fill.color == color.with_alpha(42)
            && fill.points.len() == 4
            && fill.points[0] == Point::new(150.0, 50.0 - MAP_ANCHOR_GLOW_SIZE * 0.5)));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(255, 252, 229, 245)
                    && stroke.width == 1.35
                    && stroke.points.len() == 5
                    && stroke.points[0] == Point::new(50.0, 50.0 - (MAP_SELECTED_SIZE + 6.0) * 0.5)
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(255, 255, 255, 210)
                    && stroke.width == 0.85
                    && stroke.points.len() == 5
                    && stroke.points[0] == Point::new(50.0, 50.0 - (MAP_SELECTED_SIZE + 1.5) * 0.5)
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(245, 245, 245, 220)
                    && stroke.width == 1.0
                    && stroke.points.len() == 5
                    && stroke.points[0] == Point::new(150.0, 50.0 - (MAP_ANCHOR_SIZE + 4.0) * 0.5)
        )));
    }

    #[test]
    fn hovering_starmap_node_paints_lightweight_runtime_highlight_without_label() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut item = starmap_item("/samples/kick.wav", 0.25, 0.5, color);
        item.label = String::from("Kick Tight 01");
        let mut widget = StarmapWidget::new(vec![item], StarmapViewport::default(), None);

        assert_eq!(
            widget
                .handle_input(bounds, WidgetInput::pointer_move(Point::new(50.0, 50.0)))
                .and_then(|output| output.typed_cloned::<GuiMessage>()),
            Some(GuiMessage::RememberBrowserContextMenuPointerAnchor(
                BrowserContextPointerAnchor {
                    target: BrowserContextPointerTarget::Sample(String::from("/samples/kick.wav")),
                    position: Point::new(50.0, 50.0),
                }
            ))
        );
        let mut primitives = Vec::new();
        widget.append_runtime_overlay_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert_eq!(widget.hovered_file_id.as_deref(), Some("/samples/kick.wav"));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillPolygon(fill)
                if fill.color == color.with_alpha(50)
                    && fill.points.len() == 4
                    && fill.points[0] == Point::new(50.0, 50.0 - MAP_HOVER_GLOW_SIZE * 0.5)
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(248, 248, 248, 230)
                    && stroke.points.len() == 5
        )));
        assert!(
            !primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::Text(text) if text.text.as_str() == "Kick Tight 01"
            )),
            "hovering a map sample should not paint a sample-name tooltip"
        );
    }

    #[test]
    fn focused_starmap_node_paints_highlight_without_label() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut item = starmap_item("/samples/kick.wav", 0.25, 0.5, color);
        item.label = String::from("Kick Tight 01");
        item.selected = true;
        item.focused = true;
        let widget = StarmapWidget::new(vec![item], StarmapViewport::default(), None);
        let mut primitives = Vec::new();

        widget.append_runtime_overlay_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(248, 248, 248, 190)
                    && stroke.points.len() == 5
        )));
        assert!(
            !primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::Text(text) if text.text.as_str() == "Kick Tight 01"
            )),
            "focused map samples should not paint persistent sample-name labels"
        );
        assert!(
            !primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.color == ui::Rgba8::new(248, 248, 248, 230)
            )),
            "focused selection should not paint a rectangular tooltip"
        );
    }

    #[test]
    fn active_starmap_drag_paints_current_audition_node_without_hover_label() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut item = starmap_item("/samples/kick.wav", 0.25, 0.5, color);
        item.label = String::from("Kick Tight 01");
        let widget = StarmapWidget::new(
            vec![item],
            StarmapViewport::default(),
            Some(StarmapAuditionDragState {
                last_hit_file_id: Some(String::from("/samples/kick.wav")),
                last_position: Point::new(50.0, 50.0),
                modifiers: PointerModifiers::default(),
            }),
        );
        let mut primitives = Vec::new();

        widget.append_runtime_overlay_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillPolygon(fill)
                if fill.color == color.with_alpha(70)
                    && fill.points.len() == 4
                    && fill.points[0] == Point::new(50.0, 50.0 - MAP_ACTIVE_AUDITION_GLOW_SIZE * 0.5)
        )));
        assert!(primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(stroke)
                if stroke.color == ui::Rgba8::new(255, 250, 224, 245)
                    && stroke.points.len() == 5
        )));
        assert!(
            !primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::Text(text) if text.text.as_str() == "Kick Tight 01"
            )),
            "dragging should highlight the active hit without painting hover labels"
        );
    }

    #[test]
    fn starmap_widget_synchronizes_hover_and_hit_index_from_previous_instance() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut previous = StarmapWidget::new(
            vec![starmap_item("/samples/kick.wav", 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        assert_eq!(
            previous
                .handle_input(bounds, WidgetInput::pointer_move(Point::new(50.0, 50.0)))
                .and_then(|output| output.typed_cloned::<GuiMessage>()),
            Some(GuiMessage::RememberBrowserContextMenuPointerAnchor(
                BrowserContextPointerAnchor {
                    target: BrowserContextPointerTarget::Sample(String::from("/samples/kick.wav")),
                    position: Point::new(50.0, 50.0),
                }
            )),
            "hover should remember a pointer anchor while updating local runtime state"
        );
        previous.ensure_hit_index(bounds);

        let mut next = StarmapWidget::new(
            vec![starmap_item("/samples/kick.wav", 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        next.synchronize_from_previous(&previous);

        assert_eq!(next.hovered_file_id.as_deref(), Some("/samples/kick.wav"));
        assert!(next
            .hit_index
            .matches(bounds, StarmapViewport::default(), next.item_signature));
    }

    #[test]
    fn starmap_widget_reuses_dense_base_paint_cache_from_previous_instance() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(640.0, 360.0);
        let items = (0..MAP_DENSE_ITEM_COUNT)
            .map(|index| {
                starmap_item(
                    &format!("/samples/dense-{index}.wav"),
                    (index % 100) as f32 / 100.0,
                    (index / 100) as f32 / 10.0,
                    color,
                )
            })
            .collect::<Vec<_>>();
        let previous = StarmapWidget::new(items.clone(), StarmapViewport::default(), None);
        let mut previous_primitives = Vec::new();
        previous.append_paint(
            &mut previous_primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        let previous_cached = lock_starmap_mutex(&previous.paint_cache)
            .entry
            .as_ref()
            .expect("initial paint should populate base paint cache")
            .primitives
            .clone();

        let mut next = StarmapWidget::new(
            items,
            StarmapViewport::default(),
            Some(StarmapAuditionDragState {
                last_hit_file_id: Some(String::from("/samples/dense-42.wav")),
                last_position: Point::new(100.0, 100.0),
                modifiers: PointerModifiers::default(),
            }),
        );
        next.synchronize_from_previous(&previous);
        let mut next_primitives = Vec::new();
        next.append_paint(
            &mut next_primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let next_cached = lock_starmap_mutex(&next.paint_cache)
            .entry
            .as_ref()
            .expect("synchronized widget should retain base paint cache")
            .primitives
            .clone();
        assert!(
            Arc::ptr_eq(&previous_cached, &next_cached),
            "active drag refreshes should replay cached dense node paint instead of rebuilding it"
        );
        assert!(next_primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRectBatch(batch) if batch.rects.len() == MAP_DENSE_ITEM_COUNT
        )));
    }

    #[test]
    fn starmap_widget_synchronizes_hit_scratch_from_previous_instance() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut previous = StarmapWidget::new(
            vec![starmap_item("/samples/kick.wav", 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        previous.handle_input(bounds, WidgetInput::pointer_move(Point::new(50.0, 50.0)));

        let mut next = StarmapWidget::new(
            vec![starmap_item("/samples/kick.wav", 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        next.synchronize_from_previous(&previous);

        assert!(
            Arc::ptr_eq(&previous.hit_scratch, &next.hit_scratch),
            "hit-test scratch should survive widget refreshes during dense drag playback"
        );
    }

    #[test]
    fn starmap_widget_synchronizes_drag_hit_index_for_runtime_overlay() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let file_id = String::from("/samples/kick.wav");
        let mut previous = StarmapWidget::new(
            vec![starmap_item(file_id.as_str(), 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        previous.handle_input(bounds, WidgetInput::primary_press(Point::new(50.0, 50.0)));
        assert_eq!(previous.last_hit_index, Some(0));

        let mut next = StarmapWidget::new(
            vec![starmap_item(file_id.as_str(), 0.25, 0.5, color)],
            StarmapViewport::default(),
            Some(StarmapAuditionDragState {
                last_hit_file_id: Some(file_id.clone()),
                last_position: Point::new(50.0, 50.0),
                modifiers: PointerModifiers::default(),
            }),
        );
        next.synchronize_from_previous(&previous);

        assert_eq!(next.last_hit_index, Some(0));
        assert_eq!(
            next.active_drag_item().map(|item| item.file_id.as_str()),
            Some(file_id.as_str()),
            "runtime overlay paint should reuse the synchronized hit index for active drag nodes"
        );
    }

    #[test]
    fn starmap_widget_rebuilds_hit_index_when_filtered_items_change_with_same_count() {
        let color = ui::Rgba8::new(57, 187, 245, 220);
        let bounds = Rect::from_size(200.0, 100.0);
        let mut previous = StarmapWidget::new(
            vec![starmap_item("/samples/kick.wav", 0.25, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        previous.ensure_hit_index(bounds);

        let mut next = StarmapWidget::new(
            vec![starmap_item("/samples/snare.wav", 0.75, 0.5, color)],
            StarmapViewport::default(),
            None,
        );
        next.synchronize_from_previous(&previous);
        assert!(
            !next
                .hit_index
                .matches(bounds, StarmapViewport::default(), next.item_signature),
            "same-count filtered listings must not reuse stale node cells"
        );

        next.handle_input(bounds, WidgetInput::pointer_move(Point::new(150.0, 50.0)));

        assert_eq!(next.hovered_file_id.as_deref(), Some("/samples/snare.wav"));
        assert!(next
            .hit_index
            .matches(bounds, StarmapViewport::default(), next.item_signature));
    }

    #[test]
    fn dense_starmaps_use_smaller_node_sizes() {
        assert_eq!(map_node_size(10), MAP_NODE_SIZE);
        assert_eq!(map_node_size(MAP_DENSE_ITEM_COUNT), MAP_NODE_SIZE_DENSE);
        assert_eq!(
            map_node_size(MAP_VERY_DENSE_ITEM_COUNT),
            MAP_NODE_SIZE_VERY_DENSE
        );
    }

    #[test]
    fn primary_drag_auditions_node_crossed_between_pointer_samples() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/clap.wav",
                0.5,
                0.5,
                ui::Rgba8::new(255, 160, 80, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert_eq!(
            widget
                .handle_input(bounds, WidgetInput::primary_press(Point::new(10.0, 50.0)))
                .and_then(|output| output.typed_cloned::<GuiMessage>()),
            Some(GuiMessage::BeginStarmapAuditionDrag {
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
            Some(GuiMessage::UpdateStarmapAuditionDrag {
                paths: vec![String::from("/samples/clap.wav")],
                position: Point::new(190.0, 50.0),
                modifiers: PointerModifiers::default(),
            })
        );
    }

    #[test]
    fn primary_drag_auditions_latest_node_crossed_between_pointer_samples() {
        let mut widget = StarmapWidget::new(
            vec![
                starmap_item(
                    "/samples/kick.wav",
                    0.25,
                    0.5,
                    ui::Rgba8::new(255, 160, 80, 220),
                ),
                starmap_item(
                    "/samples/snare.wav",
                    0.5,
                    0.5,
                    ui::Rgba8::new(57, 187, 245, 220),
                ),
                starmap_item(
                    "/samples/hat.wav",
                    0.75,
                    0.5,
                    ui::Rgba8::new(125, 220, 140, 220),
                ),
            ],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        widget
            .handle_input(bounds, WidgetInput::primary_press(Point::new(5.0, 50.0)))
            .expect("press starts audition drag");
        let output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(195.0, 50.0)))
            .expect("swept drag should catch the latest crossed node");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::UpdateStarmapAuditionDrag {
                paths: vec![String::from("/samples/hat.wav")],
                position: Point::new(195.0, 50.0),
                modifiers: PointerModifiers::default(),
            })
        );
    }

    #[test]
    fn primary_release_finishes_starmap_audition_drag() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        widget
            .handle_input(bounds, WidgetInput::primary_press(Point::new(50.0, 50.0)))
            .expect("primary press starts audition drag");
        let output = widget
            .handle_input(bounds, WidgetInput::primary_release(Point::new(50.0, 50.0)))
            .expect("primary release finishes audition drag");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::FinishStarmapAuditionDrag)
        );
    }

    #[test]
    /// Primary double-click on a node retriggers audition without resetting the viewport.
    fn primary_click_then_double_click_retriggers_starmap_node_without_zooming() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);
        let node_position = Point::new(50.0, 50.0);

        let first_click = widget
            .handle_input(bounds, WidgetInput::primary_press(node_position))
            .expect("primary press should audition the node")
            .typed_cloned::<GuiMessage>();
        let first_release = widget
            .handle_input(bounds, WidgetInput::primary_release(node_position))
            .expect("primary release should finish the audition click")
            .typed_cloned::<GuiMessage>();
        let double_click = widget
            .handle_input(bounds, WidgetInput::primary_double_click(node_position))
            .expect("primary double-click should retrigger the node")
            .typed_cloned::<GuiMessage>();

        assert_eq!(
            [first_click, first_release, double_click],
            [
                Some(GuiMessage::BeginStarmapAuditionDrag {
                    path: Some(String::from("/samples/kick.wav")),
                    position: node_position,
                    modifiers: PointerModifiers::default(),
                }),
                Some(GuiMessage::FinishStarmapAuditionDrag),
                Some(GuiMessage::BeginStarmapAuditionDrag {
                    path: Some(String::from("/samples/kick.wav")),
                    position: node_position,
                    modifiers: PointerModifiers::default(),
                }),
            ]
        );
    }

    #[test]
    /// Primary double-click on empty map space behaves like an empty click, not zoom reset.
    fn primary_double_click_empty_starmap_space_does_not_zoom_out() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);
        let empty_position = Point::new(180.0, 20.0);
        let output = widget
            .handle_input(bounds, WidgetInput::primary_double_click(empty_position))
            .expect("primary double-click should be handled by the starmap");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::BeginStarmapAuditionDrag {
                path: None,
                position: empty_position,
                modifiers: PointerModifiers::default(),
            })
        );
    }

    #[test]
    fn starmap_accepts_wheel_input_for_cursor_zoom() {
        let widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );

        assert!(
            widget.accepts_wheel_input(),
            "map widgets must opt into wheel routing before scroll fallback"
        );
    }

    #[test]
    fn starmap_wheel_zooms_at_pointer_position() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        let output = widget
            .handle_input(
                bounds,
                WidgetInput::plain_wheel(Point::new(50.0, 75.0), Vector2::new(0.0, -120.0)),
            )
            .expect("wheel over the map should emit a viewport zoom");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::ChangeStarmapViewport(
                StarmapViewportChange::Zoom {
                    anchor: Vector2::new(0.25, 0.75),
                    factor: 1.15,
                }
            ))
        );
    }

    #[test]
    fn secondary_drag_pans_starmap() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            None,
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert!(
            widget
                .handle_input(
                    bounds,
                    WidgetInput::pointer_press(
                        Point::new(50.0, 40.0),
                        PointerButton::Secondary,
                        PointerModifiers::default(),
                    ),
                )
                .is_none(),
            "secondary press should only arm panning"
        );
        let output = widget
            .handle_input(bounds, WidgetInput::pointer_move(Point::new(70.0, 25.0)))
            .expect("secondary drag should pan the map viewport");

        assert_eq!(
            output.typed_cloned::<GuiMessage>(),
            Some(GuiMessage::ChangeStarmapViewport(
                StarmapViewportChange::Pan {
                    delta: Vector2::new(0.1, -0.15),
                }
            ))
        );
    }

    #[test]
    fn secondary_release_does_not_finish_starmap_audition_drag() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            Some(StarmapAuditionDragState {
                last_hit_file_id: Some(String::from("/samples/kick.wav")),
                last_position: Point::new(50.0, 50.0),
                modifiers: PointerModifiers::default(),
            }),
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert!(
            widget
                .handle_input(
                    bounds,
                    WidgetInput::pointer_press(
                        Point::new(90.0, 50.0),
                        PointerButton::Secondary,
                        PointerModifiers::default(),
                    ),
                )
                .is_none(),
            "secondary press only arms map panning"
        );
        assert!(
            widget
                .handle_input(
                    bounds,
                    WidgetInput::pointer_release(
                        Point::new(90.0, 50.0),
                        PointerButton::Secondary,
                        PointerModifiers::default(),
                    ),
                )
                .is_none(),
            "secondary release must not finish the primary audition drag"
        );
    }

    #[test]
    fn secondary_drop_does_not_finish_starmap_audition_drag() {
        let mut widget = StarmapWidget::new(
            vec![starmap_item(
                "/samples/kick.wav",
                0.25,
                0.5,
                ui::Rgba8::new(57, 187, 245, 220),
            )],
            StarmapViewport::default(),
            Some(StarmapAuditionDragState {
                last_hit_file_id: Some(String::from("/samples/kick.wav")),
                last_position: Point::new(50.0, 50.0),
                modifiers: PointerModifiers::default(),
            }),
        );
        let bounds = Rect::from_size(200.0, 100.0);

        assert!(
            widget
                .handle_input(
                    bounds,
                    WidgetInput::pointer_press(
                        Point::new(90.0, 50.0),
                        PointerButton::Secondary,
                        PointerModifiers::default(),
                    ),
                )
                .is_none(),
            "secondary press should not emit a message"
        );
        assert!(
            widget
                .handle_input(
                    bounds,
                    WidgetInput::pointer_drop(
                        Point::new(90.0, 50.0),
                        PointerButton::Secondary,
                        PointerModifiers::default(),
                    ),
                )
                .is_none(),
            "secondary drop must not finish the primary audition drag"
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

    #[test]
    fn starmap_hit_index_limits_segment_candidates_to_nearby_cells() {
        let bounds = Rect::from_size(1_000.0, 1_000.0);
        let viewport = StarmapViewport::default();
        let mut items = Vec::new();
        for index in 0..2_000 {
            items.push(starmap_item(
                &format!("/samples/far-{index}.wav"),
                0.05 + (index % 20) as f32 * 0.001,
                0.05 + (index / 20) as f32 * 0.001,
                ui::Rgba8::new(255, 160, 80, 220),
            ));
        }
        items.push(starmap_item(
            "/samples/crossed.wav",
            0.75,
            0.75,
            ui::Rgba8::new(57, 187, 245, 220),
        ));

        let index =
            StarmapHitIndex::build(bounds, viewport, starmap_items_signature(&items), &items);
        let candidates = index.item_indices_near_segment(
            Point::new(720.0, 750.0),
            Point::new(780.0, 750.0),
            items.len(),
        );

        assert_eq!(candidates, vec![2_000]);
    }

    fn starmap_item(file_id: &str, x: f32, y: f32, color: ui::Rgba8) -> StarmapItem {
        StarmapItem {
            file_id: String::from(file_id),
            label: String::from(file_id),
            x,
            y,
            color,
            selected: false,
            focused: false,
            copy_flash: false,
            similarity_anchor: false,
            instant_audition_ready: true,
            missing: false,
        }
    }
}
