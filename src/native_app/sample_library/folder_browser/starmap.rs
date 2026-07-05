use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use wavecrate::sample_sources::{
    STARMAP_LAYOUT_UMAP_VERSION, StarmapLayoutLoadRequest, StarmapLayoutLoadResult,
    StarmapLayoutSample, StarmapSourceLayoutRequest,
};
use wavecrate_analysis::aspects::SimilarityAspect;

use crate::native_app::waveform::should_use_file_backed_wav_decode_for_entry;

use super::{FileEntry, FolderBrowserState, SimilarityAspectStrengths};

const STARMAP_PROJECTION_INDEX_GRID: i32 = 48;

#[derive(Clone, Copy)]
pub(in crate::native_app) struct StarmapProjection<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) instant_audition_sample_paths: &'a HashSet<String>,
    pub(in crate::native_app) preview_audition_sample_paths: &'a HashSet<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct StarmapStatus {
    pub(in crate::native_app) listed_count: usize,
    pub(in crate::native_app) layout_count: usize,
    pub(in crate::native_app) clustered_count: usize,
    pub(in crate::native_app) cluster_color_count: usize,
}

impl StarmapStatus {
    pub(in crate::native_app) fn incomplete(self) -> bool {
        self.listed_count > 0 && self.layout_count < self.listed_count
    }

    pub(in crate::native_app) fn label(self, prep_running: bool) -> Option<String> {
        if !self.incomplete() {
            return None;
        }
        if prep_running {
            return Some(format!(
                "Preparing Starmap {} / {}",
                self.layout_count, self.listed_count
            ));
        }
        Some(format!(
            "Starmap {} / {}",
            self.layout_count, self.listed_count
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct StarmapLayoutCache {
    signature: Option<u64>,
    pub(super) points_by_file: HashMap<String, StarmapLayoutPoint>,
    pub(super) projection_items: Option<Arc<[StarmapItem]>>,
    projection_index: StarmapProjectionIndex,
    listed_count: usize,
    pending_load_signature: Option<u64>,
    loaded_signature: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct StarmapProjectionIndex {
    cells: HashMap<StarmapProjectionCell, Vec<usize>>,
    file_indices: HashMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct StarmapProjectionCell {
    x: i32,
    y: i32,
}

#[derive(Debug)]
pub(in crate::native_app) struct StarmapWarmCandidateSet {
    pub(in crate::native_app) items: Arc<[StarmapItem]>,
    pub(in crate::native_app) indices: Vec<usize>,
    pub(in crate::native_app) inspected_count: usize,
    pub(in crate::native_app) cell_count: usize,
    pub(in crate::native_app) visited_cell_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct StarmapLayoutPoint {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) cluster_id: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct StarmapItem {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) x: f32,
    pub(in crate::native_app) y: f32,
    pub(in crate::native_app) color: ui::Rgba8,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) copy_flash: bool,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) instant_audition_ready: bool,
    pub(in crate::native_app) preview_audition_ready: bool,
    pub(in crate::native_app) preview_audition_candidate: bool,
    pub(in crate::native_app) missing: bool,
}

impl StarmapItem {
    pub(in crate::native_app) fn fast_audition_ready(&self) -> bool {
        self.instant_audition_ready || self.preview_audition_ready
    }

    pub(in crate::native_app) fn audition_candidate(&self) -> bool {
        self.fast_audition_ready() || self.preview_audition_candidate
    }
}

impl StarmapProjectionIndex {
    fn build(items: &[StarmapItem]) -> Self {
        let mut index = Self::default();
        for (item_index, item) in items.iter().enumerate() {
            if item.missing || !item.preview_audition_candidate {
                continue;
            }
            index.file_indices.insert(item.file_id.clone(), item_index);
            index
                .cells
                .entry(StarmapProjectionCell::for_point(item.x, item.y))
                .or_default()
                .push(item_index);
        }
        index
    }

    fn preview_warm_indices(
        &self,
        items: &[StarmapItem],
        center_x: f32,
        center_y: f32,
        zoom: f32,
        viewport_pad: f32,
        selected_file_id: Option<&str>,
        limit: usize,
    ) -> StarmapPreviewWarmScan {
        if limit == 0 {
            return StarmapPreviewWarmScan::default();
        }
        let mut indices = Vec::with_capacity(limit);
        let mut inspected_count = 0;
        let mut visited_cell_count = 0;
        let selected_index = selected_file_id.and_then(|file_id| self.file_indices.get(file_id));
        if let Some(&selected_index) = selected_index {
            indices.push(selected_index);
            inspected_count += 1;
        }

        let cells = self.viewport_cells(center_x, center_y, zoom, viewport_pad);
        let cell_count = cells.len();
        for cell in cells {
            visited_cell_count += 1;
            let Some(cell_indices) = self.cells.get(&cell) else {
                continue;
            };
            for &item_index in cell_indices {
                if Some(&item_index) == selected_index {
                    continue;
                }
                inspected_count += 1;
                let item = &items[item_index];
                if !starmap_item_in_preview_warm_viewport(
                    item.x,
                    item.y,
                    center_x,
                    center_y,
                    zoom,
                    viewport_pad,
                ) {
                    continue;
                }
                indices.push(item_index);
                if indices.len() >= limit {
                    return StarmapPreviewWarmScan {
                        indices,
                        inspected_count,
                        cell_count,
                        visited_cell_count,
                    };
                }
            }
        }
        StarmapPreviewWarmScan {
            indices,
            inspected_count,
            cell_count,
            visited_cell_count,
        }
    }

    fn viewport_cells(
        &self,
        center_x: f32,
        center_y: f32,
        zoom: f32,
        pad: f32,
    ) -> Vec<StarmapProjectionCell> {
        let (min_x, max_x, min_y, max_y) =
            StarmapProjectionCell::viewport_cell_range(center_x, center_y, zoom, pad);
        let mut cells = self
            .cells
            .keys()
            .copied()
            .filter(|cell| (min_x..=max_x).contains(&cell.x) && (min_y..=max_y).contains(&cell.y))
            .collect::<Vec<_>>();
        cells.sort_by(|left, right| {
            starmap_projection_cell_distance_sq(*left, center_x, center_y)
                .total_cmp(&starmap_projection_cell_distance_sq(
                    *right, center_x, center_y,
                ))
                .then_with(|| left.y.cmp(&right.y))
                .then_with(|| left.x.cmp(&right.x))
        });
        cells
    }
}

impl StarmapProjectionCell {
    fn for_point(x: f32, y: f32) -> Self {
        Self {
            x: starmap_projection_grid_coordinate(x),
            y: starmap_projection_grid_coordinate(y),
        }
    }

    fn viewport_cell_range(
        center_x: f32,
        center_y: f32,
        zoom: f32,
        pad: f32,
    ) -> (i32, i32, i32, i32) {
        let zoom = zoom.max(f32::EPSILON);
        let extent = (0.5 + pad.max(0.0)) / zoom;
        let min_x = starmap_projection_grid_coordinate(center_x - extent);
        let max_x = starmap_projection_grid_coordinate(center_x + extent);
        let min_y = starmap_projection_grid_coordinate(center_y - extent);
        let max_y = starmap_projection_grid_coordinate(center_y + extent);
        (min_x, max_x, min_y, max_y)
    }
}

#[derive(Debug, Default)]
struct StarmapPreviewWarmScan {
    indices: Vec<usize>,
    inspected_count: usize,
    cell_count: usize,
    visited_cell_count: usize,
}

fn starmap_projection_grid_coordinate(value: f32) -> i32 {
    (value * STARMAP_PROJECTION_INDEX_GRID as f32)
        .floor()
        .clamp(0.0, (STARMAP_PROJECTION_INDEX_GRID - 1) as f32) as i32
}

fn starmap_projection_cell_distance_sq(
    cell: StarmapProjectionCell,
    center_x: f32,
    center_y: f32,
) -> f32 {
    let cell_size = 1.0 / STARMAP_PROJECTION_INDEX_GRID as f32;
    let cell_x = (cell.x as f32 + 0.5) * cell_size;
    let cell_y = (cell.y as f32 + 0.5) * cell_size;
    let dx = cell_x - center_x;
    let dy = cell_y - center_y;
    dx * dx + dy * dy
}

fn starmap_item_in_preview_warm_viewport(
    item_x: f32,
    item_y: f32,
    center_x: f32,
    center_y: f32,
    zoom: f32,
    pad: f32,
) -> bool {
    let normalized_x = (item_x - center_x) * zoom + 0.5;
    let normalized_y = (item_y - center_y) * zoom + 0.5;
    let min = -pad;
    let max = 1.0 + pad;
    (min..=max).contains(&normalized_x) && (min..=max).contains(&normalized_y)
}

impl FolderBrowserState {
    pub(in crate::native_app) fn prepare_starmap_layout(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let signature = starmap_layout_signature(self.selected_source_id(), snapshot.rows());
        if self.sample_list.starmap_layout.signature == Some(signature) {
            return;
        }
        self.sample_list.starmap_layout = StarmapLayoutCache {
            signature: Some(signature),
            listed_count: snapshot.rows().len(),
            ..StarmapLayoutCache::default()
        };
    }

    pub(in crate::native_app) fn invalidate_starmap_layout(&mut self) {
        self.sample_list.starmap_layout = StarmapLayoutCache::default();
    }

    pub(in crate::native_app) fn take_starmap_layout_load_request(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<StarmapLayoutLoadRequest> {
        if !self.starmap_layout_load_may_need_request() {
            return None;
        }
        let (request, listed_count) = self.starmap_layout_load_request(tags_by_file);
        let signature = request.signature;
        if self.sample_list.starmap_layout.signature != Some(signature) {
            self.sample_list.starmap_layout = StarmapLayoutCache {
                signature: Some(signature),
                listed_count,
                ..StarmapLayoutCache::default()
            };
        }
        let cache = &mut self.sample_list.starmap_layout;
        if cache.pending_load_signature == Some(signature)
            || cache.loaded_signature == Some(signature)
            || request.is_empty()
        {
            if request.is_empty() {
                cache.loaded_signature = Some(signature);
            }
            return None;
        }
        cache.pending_load_signature = Some(signature);
        Some(request)
    }

    pub(in crate::native_app) fn starmap_layout_load_may_need_request(&self) -> bool {
        let cache = &self.sample_list.starmap_layout;
        let Some(signature) = cache.signature else {
            return true;
        };
        cache.pending_load_signature != Some(signature) && cache.loaded_signature != Some(signature)
    }

    pub(in crate::native_app) fn apply_starmap_layout_load_result(
        &mut self,
        result: StarmapLayoutLoadResult,
    ) {
        let cache = &mut self.sample_list.starmap_layout;
        if cache.signature != Some(result.signature) {
            return;
        }
        cache.pending_load_signature = None;
        cache.loaded_signature = Some(result.signature);
        match result.result {
            Ok(points) => {
                cache.points_by_file = points
                    .into_iter()
                    .map(|(file_id, point)| {
                        (
                            file_id,
                            StarmapLayoutPoint {
                                x: point.x,
                                y: point.y,
                                cluster_id: point.cluster_id,
                            },
                        )
                    })
                    .collect();
            }
            Err(error) => {
                tracing::debug!(%error, "starmap layout unavailable");
                cache.points_by_file.clear();
            }
        }
    }

    pub(in crate::native_app) fn starmap_projection(
        &self,
        projection: StarmapProjection<'_>,
    ) -> Vec<StarmapItem> {
        self.build_starmap_projection(projection)
    }

    pub(in crate::native_app) fn prepare_starmap_projection(
        &mut self,
        projection: StarmapProjection<'_>,
    ) {
        let items = Arc::<[StarmapItem]>::from(self.build_starmap_projection(projection));
        self.sample_list.starmap_layout.projection_index = StarmapProjectionIndex::build(&items);
        self.sample_list.starmap_layout.projection_items = Some(items);
    }

    pub(in crate::native_app) fn cached_starmap_projection(&self) -> Option<Arc<[StarmapItem]>> {
        self.sample_list.starmap_layout.projection_items.clone()
    }

    pub(in crate::native_app) fn cached_starmap_projection_len(&self) -> usize {
        self.sample_list
            .starmap_layout
            .projection_items
            .as_ref()
            .map_or(0, |items| items.len())
    }

    pub(in crate::native_app) fn cached_starmap_preview_warm_candidates(
        &self,
        center_x: f32,
        center_y: f32,
        zoom: f32,
        viewport_pad: f32,
        selected_file_id: Option<&str>,
        limit: usize,
    ) -> Option<StarmapWarmCandidateSet> {
        let cache = &self.sample_list.starmap_layout;
        let items = cache.projection_items.clone()?;
        let scan = cache.projection_index.preview_warm_indices(
            &items,
            center_x,
            center_y,
            zoom,
            viewport_pad,
            selected_file_id,
            limit,
        );
        Some(StarmapWarmCandidateSet {
            items,
            indices: scan.indices,
            inspected_count: scan.inspected_count,
            cell_count: scan.cell_count,
            visited_cell_count: scan.visited_cell_count,
        })
    }

    fn build_starmap_projection(&self, projection: StarmapProjection<'_>) -> Vec<StarmapItem> {
        let snapshot = self.browser_listing_snapshot(projection.tags_by_file);
        let focused_file_id = self.selected_file_id();
        snapshot
            .rows()
            .iter()
            .filter_map(|file| {
                let layout_point = self
                    .sample_list
                    .starmap_layout
                    .points_by_file
                    .get(&file.id)
                    .copied()?;
                let instant_audition_ready = instant_audition_ready_for_starmap(
                    file,
                    projection.instant_audition_sample_paths,
                );
                let preview_audition_ready =
                    projection.preview_audition_sample_paths.contains(&file.id);
                let aspects = self.similarity_aspect_display_strengths_for_file(&file.id);
                let strength = self.similarity_display_strength_for_file(&file.id);
                let group = strongest_enabled_aspect(&aspects, self.similarity_controls());
                let (x, y) = starmap_position(layout_point);
                Some(StarmapItem {
                    file_id: file.id.clone(),
                    label: file.stem.clone(),
                    x,
                    y,
                    color: starmap_color(group, strength, Some(layout_point)),
                    selected: self.is_file_selected(&file.id),
                    focused: focused_file_id == Some(file.id.as_str()),
                    copy_flash: self.copied_file_flash_active(&file.id),
                    similarity_anchor: self.file_is_similarity_anchor(&file.id),
                    instant_audition_ready,
                    preview_audition_ready,
                    preview_audition_candidate: preview_audition_candidate_for_starmap(file),
                    missing: file.is_missing(),
                })
            })
            .collect()
    }

    pub(in crate::native_app) fn selected_starmap_position(
        &self,
        projection: StarmapProjection<'_>,
    ) -> Option<(f32, f32)> {
        let selected_file = self.selected_file_id()?;
        if let Some(items) = self.cached_starmap_projection()
            && let Some(item) = find_starmap_item_by_file_id(&items, selected_file)
        {
            return Some((item.x, item.y));
        }
        self.starmap_projection(projection)
            .into_iter()
            .find(|item| item.file_id == selected_file)
            .map(|item| (item.x, item.y))
    }

    pub(in crate::native_app) fn navigate_starmap_matching_tags(
        &mut self,
        delta: i32,
        extend: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
        instant_audition_sample_paths: &HashSet<String>,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() || !self.selection.selected_file_active() {
            return None;
        }
        self.prepare_starmap_layout(tags_by_file);
        let cached_projection = self.cached_starmap_projection();
        let built_projection;
        let projection_items = if let Some(items) = cached_projection.as_deref() {
            items
        } else {
            built_projection = self.starmap_projection(StarmapProjection {
                tags_by_file,
                instant_audition_sample_paths,
                preview_audition_sample_paths: &HashSet::new(),
            });
            &built_projection
        };
        let target =
            starmap_navigation_target(projection_items, self.selection.selected_file_id()?, delta)?;
        let visible_ids = self.browser_listing_snapshot(tags_by_file).ids().to_vec();
        if extend {
            self.selection.select_file_with_modifiers(
                target.clone(),
                &visible_ids,
                PointerModifiers {
                    shift: true,
                    ..PointerModifiers::default()
                },
            );
        } else {
            self.selection
                .navigate_file_to_adjacent_visible_id(target.clone())?;
        }
        Some(target)
    }

    pub(in crate::native_app) fn starmap_status(&self) -> StarmapStatus {
        let clustered_count = self
            .sample_list
            .starmap_layout
            .points_by_file
            .values()
            .filter(|point| point.cluster_id.is_some())
            .count();
        let cluster_color_count = if clustered_count == 0 {
            0
        } else {
            clustered_count.min(STARMAP_CLUSTER_PALETTE.len())
        };
        StarmapStatus {
            listed_count: self.sample_list.starmap_layout.listed_count,
            layout_count: self.sample_list.starmap_layout.points_by_file.len(),
            clustered_count,
            cluster_color_count,
        }
    }
}

fn starmap_navigation_target(
    items: &[StarmapItem],
    selected_file_id: &str,
    delta: i32,
) -> Option<String> {
    let current = find_starmap_item_by_file_id(items, selected_file_id)?;
    let direction = delta.signum() as f32;
    items
        .iter()
        .filter(|item| item.file_id != current.file_id)
        .filter(|item| (item.y - current.y) * direction > f32::EPSILON)
        .min_by(|left, right| {
            starmap_navigation_rank(current, left)
                .total_cmp(&starmap_navigation_rank(current, right))
                .then_with(|| left.file_id.cmp(&right.file_id))
        })
        .map(|item| item.file_id.clone())
}

fn find_starmap_item_by_file_id<'a>(
    items: &'a [StarmapItem],
    file_id: &str,
) -> Option<&'a StarmapItem> {
    items.iter().find(|item| item.file_id.as_str() == file_id)
}

fn starmap_navigation_rank(current: &StarmapItem, candidate: &StarmapItem) -> f32 {
    let dx = candidate.x - current.x;
    let dy = candidate.y - current.y;
    dx * dx + dy * dy
}

fn instant_audition_ready_for_starmap(
    file: &FileEntry,
    instant_audition_sample_paths: &HashSet<String>,
) -> bool {
    !should_use_file_backed_wav_decode_for_entry(&file.extension, file.size_bytes)
        || instant_audition_sample_paths.contains(&file.id)
}

fn preview_audition_candidate_for_starmap(file: &FileEntry) -> bool {
    file.extension.eq_ignore_ascii_case("wav") || file.extension.eq_ignore_ascii_case("wave")
}

impl FolderBrowserState {
    fn starmap_layout_load_request(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> (StarmapLayoutLoadRequest, usize) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let listed_count = snapshot.rows().len();
        let mut by_source: HashMap<String, StarmapSourceLayoutRequest> = HashMap::new();
        for file in snapshot.rows() {
            let path = Path::new(&file.id);
            let Some((source, relative_path)) = self.sample_source_for_file_path(path) else {
                continue;
            };
            let sample_id = build_sample_id(source.id.as_str(), &relative_path);
            by_source
                .entry(source.id.as_str().to_string())
                .or_insert_with(|| StarmapSourceLayoutRequest {
                    source,
                    samples: Vec::new(),
                })
                .samples
                .push(StarmapLayoutSample {
                    file_id: file.id.clone(),
                    sample_id,
                });
        }
        let signature = starmap_layout_signature(self.selected_source_id(), snapshot.rows());
        (
            StarmapLayoutLoadRequest {
                signature,
                sources: by_source.into_values().collect(),
            },
            listed_count,
        )
    }
}

fn build_sample_id(source_id: &str, relative_path: &Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

fn starmap_layout_signature(source_id: &str, files: &[&FileEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_id.hash(&mut hasher);
    STARMAP_LAYOUT_UMAP_VERSION.hash(&mut hasher);
    files.len().hash(&mut hasher);
    for file in files {
        file.id.hash(&mut hasher);
    }
    hasher.finish()
}

fn strongest_enabled_aspect(
    aspects: &SimilarityAspectStrengths,
    controls: &wavecrate::sample_sources::config::SimilarityAspectSettings,
) -> SimilarityAspect {
    let enabled = controls.aspect_enabled_flags();
    SimilarityAspect::ORDER
        .iter()
        .copied()
        .filter(|aspect| enabled[aspect.index()])
        .filter(|aspect| *aspect != SimilarityAspect::Overall)
        .max_by(|left, right| {
            aspect_strength(aspects, *left)
                .partial_cmp(&aspect_strength(aspects, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(SimilarityAspect::Overall)
}

fn aspect_strength(aspects: &SimilarityAspectStrengths, aspect: SimilarityAspect) -> f32 {
    aspects
        .get(aspect.index())
        .copied()
        .flatten()
        .unwrap_or(0.0)
}

fn starmap_position(layout_point: StarmapLayoutPoint) -> (f32, f32) {
    (layout_point.x, layout_point.y)
}

fn starmap_color(
    group: SimilarityAspect,
    strength: Option<f32>,
    layout_point: Option<StarmapLayoutPoint>,
) -> ui::Rgba8 {
    if let Some(point) = layout_point
        && point.cluster_id.is_some()
    {
        return starmap_cluster_color((point.x, point.y), strength);
    }
    let alpha = (150.0 + strength.unwrap_or(0.35).clamp(0.0, 1.0) * 90.0) as u8;
    match group {
        SimilarityAspect::Overall => ui::Rgba8::new(122, 226, 96, alpha),
        SimilarityAspect::Spectrum => ui::Rgba8::new(239, 216, 66, alpha),
        SimilarityAspect::Timbre => ui::Rgba8::new(255, 142, 56, alpha),
        SimilarityAspect::Pitch => ui::Rgba8::new(255, 55, 96, alpha),
        SimilarityAspect::Amplitude => ui::Rgba8::new(57, 187, 245, alpha),
    }
}

fn starmap_cluster_color(position: (f32, f32), strength: Option<f32>) -> ui::Rgba8 {
    let alpha = (180.0 + strength.unwrap_or(0.45).clamp(0.0, 1.0) * 60.0) as u8;
    blended_starmap_cluster_color(position).with_alpha(alpha)
}

pub(in crate::native_app) fn starmap_cluster_palette_color(index: usize) -> ui::Rgba8 {
    STARMAP_CLUSTER_PALETTE[index % STARMAP_CLUSTER_PALETTE.len()]
}

fn blended_starmap_cluster_color(position: (f32, f32)) -> ui::Rgba8 {
    let mut total = 0.0;
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    for anchor in STARMAP_CLUSTER_COLOR_ANCHORS {
        let dx = position.0 - anchor.x;
        let dy = position.1 - anchor.y;
        let weight = 1.0 / (dx * dx + dy * dy + 0.025).powf(1.6);
        total += weight;
        red += f32::from(anchor.color.r) * weight;
        green += f32::from(anchor.color.g) * weight;
        blue += f32::from(anchor.color.b) * weight;
    }
    ui::Rgba8::new(
        blended_color_channel(red, total),
        blended_color_channel(green, total),
        blended_color_channel(blue, total),
        230,
    )
}

fn blended_color_channel(weighted: f32, total: f32) -> u8 {
    if total <= f32::EPSILON {
        return 0;
    }
    (weighted / total).round().clamp(0.0, 255.0) as u8
}

#[derive(Clone, Copy)]
struct StarmapClusterColorAnchor {
    x: f32,
    y: f32,
    color: ui::Rgba8,
}

const STARMAP_CLUSTER_COLOR_ANCHORS: [StarmapClusterColorAnchor; 5] = [
    StarmapClusterColorAnchor {
        x: 0.16,
        y: 0.46,
        color: STARMAP_CLUSTER_PALETTE[0],
    },
    StarmapClusterColorAnchor {
        x: 0.36,
        y: 0.24,
        color: STARMAP_CLUSTER_PALETTE[1],
    },
    StarmapClusterColorAnchor {
        x: 0.52,
        y: 0.52,
        color: STARMAP_CLUSTER_PALETTE[2],
    },
    StarmapClusterColorAnchor {
        x: 0.68,
        y: 0.34,
        color: STARMAP_CLUSTER_PALETTE[3],
    },
    StarmapClusterColorAnchor {
        x: 0.84,
        y: 0.62,
        color: STARMAP_CLUSTER_PALETTE[4],
    },
];

const STARMAP_CLUSTER_PALETTE: [ui::Rgba8; 5] = [
    ui::Rgba8::new(255, 55, 96, 230),
    ui::Rgba8::new(114, 235, 184, 230),
    ui::Rgba8::new(255, 179, 92, 230),
    ui::Rgba8::new(186, 91, 255, 230),
    ui::Rgba8::new(57, 187, 245, 230),
];

#[cfg(test)]
mod tests {
    use super::*;
    use wavecrate::sample_sources::SampleSource;

    fn test_starmap_item(file_id: &str, x: f32, y: f32) -> StarmapItem {
        StarmapItem {
            file_id: file_id.to_string(),
            label: file_id.to_string(),
            x,
            y,
            color: ui::Rgba8::new(57, 187, 245, 220),
            selected: false,
            focused: false,
            copy_flash: false,
            similarity_anchor: false,
            instant_audition_ready: true,
            preview_audition_ready: false,
            preview_audition_candidate: true,
            missing: false,
        }
    }

    #[test]
    fn starmap_projection_index_returns_nearby_preview_warm_candidates_without_full_scan() {
        let mut items = Vec::new();
        for index in 0..2_000 {
            items.push(test_starmap_item(
                &format!("far-{index:04}.wav"),
                0.94,
                0.94,
            ));
        }
        for index in 0..96 {
            let offset = (index as f32 % 12.0) * 0.0007;
            items.push(test_starmap_item(
                &format!("near-{index:03}.wav"),
                0.498 + offset,
                0.502 + offset,
            ));
        }
        let items = Arc::<[StarmapItem]>::from(items);
        let index = StarmapProjectionIndex::build(&items);

        let scan = index.preview_warm_indices(&items, 0.5, 0.5, 32.0, 0.08, None, 24);

        assert_eq!(scan.indices.len(), 24);
        assert!(
            scan.inspected_count < 128,
            "zoomed dense warm planning should visit nearby cells, not all {} items",
            items.len()
        );
        assert!(
            scan.indices
                .iter()
                .all(|&item_index| items[item_index].file_id.starts_with("near-"))
        );
        assert!(
            scan.visited_cell_count <= scan.cell_count,
            "visited cells should be bounded by occupied viewport cells"
        );
        assert!(
            scan.cell_count < STARMAP_PROJECTION_INDEX_GRID as usize,
            "zoomed warm planning should not sort the whole projection grid"
        );
    }

    #[test]
    fn starmap_projection_index_skips_empty_preview_warm_cells() {
        let items = Arc::<[StarmapItem]>::from(vec![
            test_starmap_item("center.wav", 0.50, 0.50),
            test_starmap_item("top-left.wav", 0.05, 0.05),
            test_starmap_item("bottom-right.wav", 0.95, 0.95),
        ]);
        let index = StarmapProjectionIndex::build(&items);

        let scan = index.preview_warm_indices(&items, 0.5, 0.5, 1.0, 0.0, None, 1);

        assert_eq!(scan.indices.len(), 1);
        assert_eq!(
            scan.cell_count,
            index.cells.len(),
            "full-map warm planning should sort occupied cells, not every empty grid cell"
        );
        assert_eq!(
            scan.visited_cell_count, 1,
            "warm planning should stop walking cells once the requested candidates are found"
        );
    }

    #[test]
    fn starmap_projection_index_keeps_selected_preview_warm_candidate_first() {
        let items = Arc::<[StarmapItem]>::from(vec![
            test_starmap_item("selected.wav", 0.95, 0.95),
            test_starmap_item("near.wav", 0.5, 0.5),
        ]);
        let index = StarmapProjectionIndex::build(&items);

        let scan =
            index.preview_warm_indices(&items, 0.5, 0.5, 32.0, 0.08, Some("selected.wav"), 2);

        assert_eq!(scan.indices, vec![0, 1]);
        assert_eq!(scan.inspected_count, 2);
    }

    fn write_sparse_wav_i16(path: &Path, channels: u16, frames: u32) {
        let channels = channels.max(1);
        let sample_rate = 48_000_u32;
        let bits_per_sample = 16_u16;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let data_bytes = frames
            .checked_mul(u32::from(block_align))
            .expect("test wav data size");
        let riff_size = 36_u32.checked_add(data_bytes).expect("test wav riff size");
        let mut file = std::fs::File::create(path).expect("create sparse wav");
        use std::io::Write;
        file.write_all(b"RIFF").expect("write riff");
        file.write_all(&riff_size.to_le_bytes())
            .expect("write riff size");
        file.write_all(b"WAVE").expect("write wave");
        file.write_all(b"fmt ").expect("write fmt");
        file.write_all(&16_u32.to_le_bytes())
            .expect("write fmt size");
        file.write_all(&1_u16.to_le_bytes())
            .expect("write pcm format");
        file.write_all(&channels.to_le_bytes())
            .expect("write channels");
        file.write_all(&sample_rate.to_le_bytes())
            .expect("write sample rate");
        file.write_all(&byte_rate.to_le_bytes())
            .expect("write byte rate");
        file.write_all(&block_align.to_le_bytes())
            .expect("write block align");
        file.write_all(&bits_per_sample.to_le_bytes())
            .expect("write bits");
        file.write_all(b"data").expect("write data chunk");
        file.write_all(&data_bytes.to_le_bytes())
            .expect("write data size");
        file.set_len(44_u64 + u64::from(data_bytes))
            .expect("extend sparse wav");
    }

    fn test_layout_point(
        index: usize,
        total: usize,
    ) -> wavecrate::sample_sources::StarmapLayoutPoint {
        let total = total.max(1) as f32;
        let t = (index as f32 + 0.5) / total;
        wavecrate::sample_sources::StarmapLayoutPoint {
            x: 0.10 + 0.80 * t,
            y: 0.50 + ((index % 5) as f32 - 2.0) * 0.04,
            cluster_id: None,
        }
    }

    fn complete_test_starmap_layout(
        browser: &mut FolderBrowserState,
        tags_by_file: &HashMap<String, Vec<String>>,
        file_ids: &[String],
    ) {
        browser.prepare_starmap_layout(tags_by_file);
        let request = browser
            .take_starmap_layout_load_request(tags_by_file)
            .expect("starmap layout request");
        browser.apply_starmap_layout_load_result(StarmapLayoutLoadResult {
            signature: request.signature,
            result: Ok(file_ids
                .iter()
                .enumerate()
                .map(|(index, file_id)| (file_id.clone(), test_layout_point(index, file_ids.len())))
                .collect()),
        });
    }

    #[test]
    fn starmap_position_uses_normalized_layout_when_available() {
        let position = starmap_position(StarmapLayoutPoint {
            x: 0.25,
            y: 0.75,
            cluster_id: None,
        });

        assert_eq!(position, (0.25, 0.75));
    }

    #[test]
    fn starmap_projection_omits_missing_layout_rows_before_and_after_load_completes() {
        let root = tempfile::tempdir().expect("source root");
        let positioned = root.path().join("positioned.wav");
        let missing = root.path().join("missing.wav");
        std::fs::write(&positioned, []).expect("write positioned");
        std::fs::write(&missing, []).expect("write missing");
        let positioned_id = positioned.to_string_lossy().to_string();
        let missing_id = missing.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        let request = browser
            .take_starmap_layout_load_request(&tags_by_file)
            .expect("layout request");
        assert!(
            browser
                .starmap_projection(StarmapProjection {
                    tags_by_file: &tags_by_file,
                    instant_audition_sample_paths: &HashSet::new(),
                    preview_audition_sample_paths: &HashSet::new(),
                })
                .is_empty(),
            "pending Starmap loads should not draw synthetic fallback positions for missing layout rows"
        );
        browser.apply_starmap_layout_load_result(StarmapLayoutLoadResult {
            signature: request.signature,
            result: Ok(HashMap::from([(
                positioned_id.clone(),
                wavecrate::sample_sources::StarmapLayoutPoint {
                    x: 0.25,
                    y: 0.75,
                    cluster_id: None,
                },
            )])),
        });

        let map_ids = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();

        assert_eq!(map_ids, vec![positioned_id]);
        assert!(
            !map_ids.contains(&missing_id),
            "completed Starmap loads should keep omitting missing layout rows"
        );
    }

    #[test]
    fn starmap_color_prefers_similarity_cluster_color() {
        let cluster_color = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.16,
                y: 0.46,
                cluster_id: Some(1),
            }),
        );
        let aspect_color = starmap_color(SimilarityAspect::Spectrum, Some(0.5), None);

        assert_ne!(cluster_color, aspect_color);
        assert_eq!(cluster_color.a, 210);
    }

    #[test]
    fn starmap_cluster_colors_fade_by_layout_position() {
        let left = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.16,
                y: 0.46,
                cluster_id: Some(1),
            }),
        );
        let nearby = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.20,
                y: 0.48,
                cluster_id: Some(37),
            }),
        );
        let far = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.84,
                y: 0.62,
                cluster_id: Some(37),
            }),
        );

        assert!(
            color_distance(left, nearby) < color_distance(left, far),
            "nearby clustered samples should have more similar colors than distant samples"
        );
    }

    #[test]
    fn selected_starmap_position_uses_current_filtered_projection() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        std::fs::write(&kick, []).expect("write sample");
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let kick_id = kick.to_string_lossy().to_string();
        browser.select_file(kick_id.clone());
        let tags_by_file = HashMap::new();
        complete_test_starmap_layout(&mut browser, &tags_by_file, std::slice::from_ref(&kick_id));

        let position = browser.selected_starmap_position(StarmapProjection {
            tags_by_file: &tags_by_file,
            instant_audition_sample_paths: &HashSet::new(),
            preview_audition_sample_paths: &HashSet::new(),
        });

        assert!(position.is_some());
        let projection = browser.starmap_projection(StarmapProjection {
            tags_by_file: &tags_by_file,
            instant_audition_sample_paths: &HashSet::new(),
            preview_audition_sample_paths: &HashSet::new(),
        });
        let selected = projection
            .iter()
            .find(|item| item.file_id == kick_id)
            .expect("selected map item");
        assert_eq!(position, Some((selected.x, selected.y)));
        assert!(selected.focused);
    }

    #[test]
    fn selected_starmap_position_reuses_cached_projection() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        std::fs::write(&kick, []).expect("write sample");
        let kick_id = kick.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        browser.select_file(kick_id.clone());
        browser.sample_list.starmap_layout.projection_items =
            Some(Arc::from(vec![test_starmap_item(&kick_id, 0.18, 0.82)]));
        let tags_by_file = HashMap::new();

        let position = browser.selected_starmap_position(StarmapProjection {
            tags_by_file: &tags_by_file,
            instant_audition_sample_paths: &HashSet::new(),
            preview_audition_sample_paths: &HashSet::new(),
        });

        assert_eq!(position, Some((0.18, 0.82)));
    }

    #[test]
    fn starmap_layout_request_is_needed_only_until_pending_or_loaded() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        std::fs::write(&kick, []).expect("write sample");
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        browser.prepare_starmap_layout(&tags_by_file);

        assert!(browser.starmap_layout_load_may_need_request());
        let request = browser
            .take_starmap_layout_load_request(&tags_by_file)
            .expect("first map layout request");

        assert!(
            !browser.starmap_layout_load_may_need_request(),
            "pending layout loads should not ask the frame loop to rebuild map requests"
        );
        assert!(
            browser
                .take_starmap_layout_load_request(&tags_by_file)
                .is_none(),
            "duplicate layout requests should stay suppressed while one is pending"
        );
        browser.apply_starmap_layout_load_result(StarmapLayoutLoadResult {
            signature: request.signature,
            result: Ok(HashMap::new()),
        });
        assert!(
            !browser.starmap_layout_load_may_need_request(),
            "loaded layouts should stay quiet until the starmap layout is invalidated"
        );

        browser.invalidate_starmap_layout();
        assert!(browser.starmap_layout_load_may_need_request());
    }

    #[test]
    fn starmap_keyboard_navigation_uses_map_position_not_list_order() {
        let root = tempfile::tempdir().expect("source root");
        let alpha = root.path().join("alpha.wav");
        let beta = root.path().join("beta.wav");
        let close_below = root.path().join("close_below.wav");
        std::fs::write(&alpha, []).expect("write alpha");
        std::fs::write(&beta, []).expect("write beta");
        std::fs::write(&close_below, []).expect("write close");
        let alpha_id = alpha.to_string_lossy().to_string();
        let beta_id = beta.to_string_lossy().to_string();
        let close_below_id = close_below.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        browser.prepare_starmap_layout(&tags_by_file);
        browser.sample_list.starmap_layout.points_by_file = HashMap::from([
            (
                alpha_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.50,
                    cluster_id: None,
                },
            ),
            (
                beta_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.92,
                    cluster_id: None,
                },
            ),
            (
                close_below_id.clone(),
                StarmapLayoutPoint {
                    x: 0.52,
                    y: 0.58,
                    cluster_id: None,
                },
            ),
        ]);
        browser.select_file(alpha_id.clone());

        let down = browser.navigate_starmap_matching_tags(1, false, &tags_by_file, &HashSet::new());
        let up = browser.navigate_starmap_matching_tags(-1, false, &tags_by_file, &HashSet::new());

        assert_eq!(
            down,
            Some(close_below_id),
            "map navigation should pick the closest lower map node, not the next filename row"
        );
        assert_eq!(up, Some(alpha_id));
    }

    #[test]
    fn starmap_keyboard_navigation_reuses_cached_projection() {
        let root = tempfile::tempdir().expect("source root");
        let alpha = root.path().join("alpha.wav");
        let beta = root.path().join("beta.wav");
        let close_below = root.path().join("close_below.wav");
        std::fs::write(&alpha, []).expect("write alpha");
        std::fs::write(&beta, []).expect("write beta");
        std::fs::write(&close_below, []).expect("write close");
        let alpha_id = alpha.to_string_lossy().to_string();
        let beta_id = beta.to_string_lossy().to_string();
        let close_below_id = close_below.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        browser.prepare_starmap_layout(&tags_by_file);
        browser.sample_list.starmap_layout.points_by_file = HashMap::from([
            (
                alpha_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.50,
                    cluster_id: None,
                },
            ),
            (
                beta_id.clone(),
                StarmapLayoutPoint {
                    x: 0.51,
                    y: 0.58,
                    cluster_id: None,
                },
            ),
            (
                close_below_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.92,
                    cluster_id: None,
                },
            ),
        ]);
        browser.sample_list.starmap_layout.projection_items = Some(Arc::from(vec![
            test_starmap_item(&alpha_id, 0.50, 0.50),
            test_starmap_item(&beta_id, 0.50, 0.92),
            test_starmap_item(&close_below_id, 0.52, 0.58),
        ]));
        browser.select_file(alpha_id);

        let down = browser.navigate_starmap_matching_tags(1, false, &tags_by_file, &HashSet::new());

        assert_eq!(
            down,
            Some(close_below_id),
            "keyboard navigation should use the already prepared map projection instead of rebuilding dense items"
        );
    }

    #[test]
    fn starmap_projection_matches_filtered_browser_listing() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("deep_kick.wav");
        let snare = root.path().join("deep_snare.wav");
        let hat = root.path().join("bright_hat.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        std::fs::write(&hat, []).expect("write hat");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let hat_id = hat.to_string_lossy().to_string();
        let tags_by_file = HashMap::from([
            (kick_id.clone(), vec![String::from("drum")]),
            (snare_id.clone(), vec![String::from("drum")]),
            (hat_id.clone(), vec![String::from("metal")]),
        ]);
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);

        browser.apply_name_filter_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("deep"),
        });
        browser.apply_tag_filter_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("drum"),
        });
        complete_test_starmap_layout(
            &mut browser,
            &tags_by_file,
            &[kick_id.clone(), snare_id.clone()],
        );

        let listing_ids = browser
            .browser_listing_snapshot(&tags_by_file)
            .ids()
            .to_vec();
        let map_ids = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();

        assert_eq!(listing_ids, vec![kick_id, snare_id]);
        assert_eq!(
            map_ids, listing_ids,
            "starmap mode must project exactly the same filtered files as list mode"
        );
    }

    #[test]
    fn starmap_projection_uses_full_filtered_listing_not_virtual_list_window() {
        let root = tempfile::tempdir().expect("source root");
        let files = (0..32)
            .map(|index| root.path().join(format!("drum_{index:02}.wav")))
            .collect::<Vec<_>>();
        for file in &files {
            std::fs::write(file, []).expect("write sample");
        }
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
            offset_y: 0.0,
            row_height: 22.0,
            window: radiant::prelude::VirtualListWindow {
                total_items: 32,
                viewport_start: 0,
                viewport_end: 8,
                window_start: 0,
                window_end: 8,
            },
        });
        let tags_by_file = HashMap::new();
        let cached_sample_paths = HashSet::new();
        let file_ids = files
            .iter()
            .map(|file| file.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        complete_test_starmap_layout(&mut browser, &tags_by_file, &file_ids);

        let visible = browser.visible_samples(
            crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery {
                tags_by_file: &tags_by_file,
                cached_sample_paths: &cached_sample_paths,
            },
        );
        let map_ids = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();
        let listing_ids = browser
            .browser_listing_snapshot(&tags_by_file)
            .ids()
            .to_vec();

        assert!(visible.rows.len() < visible.total_count);
        assert_eq!(visible.rows.len(), 8);
        assert_eq!(visible.total_count, 32);
        assert_eq!(
            map_ids, listing_ids,
            "starmap must include the full filtered listing, not only virtualized list rows"
        );
    }

    #[test]
    fn starmap_projection_marks_cold_long_wavs_as_preview_candidates() {
        let root = tempfile::tempdir().expect("source root");
        let short = root.path().join("short.wav");
        let long = root.path().join("long.wav");
        std::fs::write(&short, []).expect("write short sample");
        write_sparse_wav_i16(&long, 1, 1_024);
        let short_id = short.to_string_lossy().to_string();
        let long_id = long.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        complete_test_starmap_layout(
            &mut browser,
            &tags_by_file,
            &[long_id.clone(), short_id.clone()],
        );

        let cold_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| {
                let audition_candidate = item.audition_candidate();
                (
                    item.file_id,
                    item.instant_audition_ready,
                    item.preview_audition_candidate,
                    audition_candidate,
                )
            })
            .collect::<Vec<_>>();
        let ready_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::from([long_id.clone()]),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| {
                let audition_candidate = item.audition_candidate();
                (
                    item.file_id,
                    item.instant_audition_ready,
                    item.preview_audition_candidate,
                    audition_candidate,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            cold_items,
            vec![
                (long_id.clone(), false, true, true),
                (short_id.clone(), true, true, true)
            ]
        );
        assert_eq!(
            ready_items,
            vec![(long_id, true, true, true), (short_id, true, true, true)]
        );
    }

    #[test]
    fn starmap_projection_marks_preview_heads_fast_ready_without_full_cache() {
        let root = tempfile::tempdir().expect("source root");
        let long = root.path().join("long.wav");
        write_sparse_wav_i16(&long, 1, 1_024);
        let long_id = long.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        complete_test_starmap_layout(&mut browser, &tags_by_file, std::slice::from_ref(&long_id));

        let item = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::from([long_id.clone()]),
            })
            .into_iter()
            .find(|item| item.file_id == long_id)
            .expect("long map item");

        assert!(!item.instant_audition_ready);
        assert!(item.preview_audition_ready);
        assert!(item.fast_audition_ready());
    }

    #[test]
    fn starmap_projection_groups_by_enabled_similarity_aspects() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        let snare = root.path().join("snare.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let mut aspects = [None; wavecrate_analysis::aspects::ASPECT_COUNT];
        aspects[SimilarityAspect::Spectrum.index()] = Some(0.6);
        aspects[SimilarityAspect::Timbre.index()] = Some(1.0);
        browser.set_similarity_scores_with_aspects(
            kick_id.clone(),
            HashMap::from([(snare_id.clone(), 0.9)]),
            HashMap::from([(snare_id.clone(), aspects)]),
        );
        let tags_by_file = HashMap::new();
        complete_test_starmap_layout(
            &mut browser,
            &tags_by_file,
            &[kick_id.clone(), snare_id.clone()],
        );

        let timbre_color = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .find(|item| item.file_id == snare_id.as_str())
            .expect("snare map item")
            .color;

        let mut controls = browser.similarity_controls().clone();
        controls.set_aspect_enabled(SimilarityAspect::Timbre, false);
        browser.set_similarity_controls(controls);
        let spectrum_color = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .find(|item| item.file_id == snare_id.as_str())
            .expect("snare map item after disabling timbre")
            .color;

        assert_eq!(
            (timbre_color.r, timbre_color.g, timbre_color.b),
            (255, 142, 56)
        );
        assert_eq!(
            (spectrum_color.r, spectrum_color.g, spectrum_color.b),
            (239, 216, 66)
        );
    }

    #[test]
    fn starmap_projection_marks_all_selected_list_items() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        let snare = root.path().join("snare.wav");
        let hat = root.path().join("hat.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        std::fs::write(&hat, []).expect("write hat");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let hat_id = hat.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        complete_test_starmap_layout(
            &mut browser,
            &tags_by_file,
            &[kick_id.clone(), snare_id.clone(), hat_id.clone()],
        );

        browser.select_file(kick_id.clone());
        browser.select_file_with_modifiers(
            snare_id.clone(),
            radiant::widgets::PointerModifiers {
                command: true,
                ..radiant::widgets::PointerModifiers::default()
            },
        );

        let selected_map_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
                preview_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .filter(|item| item.selected)
            .collect::<Vec<_>>();
        let selected_map_ids = selected_map_items
            .iter()
            .map(|item| item.file_id.clone())
            .collect::<Vec<_>>();
        let focused_map_ids = selected_map_items
            .iter()
            .filter(|item| item.focused)
            .map(|item| item.file_id.clone())
            .collect::<Vec<_>>();

        assert_eq!(selected_map_ids, vec![kick_id, snare_id.clone()]);
        assert_eq!(focused_map_ids, vec![snare_id]);
        assert!(!selected_map_ids.contains(&hat_id));
    }

    #[test]
    fn starmap_status_reports_incomplete_layout_coverage() {
        let status = StarmapStatus {
            listed_count: 12,
            layout_count: 5,
            clustered_count: 2,
            cluster_color_count: 2,
        };

        assert!(status.incomplete());
        assert_eq!(
            status.label(true),
            Some(String::from("Preparing Starmap 5 / 12"))
        );
        assert_eq!(status.label(false), Some(String::from("Starmap 5 / 12")));
    }

    #[test]
    fn complete_starmap_status_stays_silent() {
        let status = StarmapStatus {
            listed_count: 12,
            layout_count: 12,
            clustered_count: 8,
            cluster_color_count: 4,
        };

        assert!(!status.incomplete());
        assert_eq!(status.label(true), None);
    }

    #[test]
    fn strongest_enabled_aspect_uses_similarity_strengths() {
        let mut aspects = [None; wavecrate_analysis::aspects::ASPECT_COUNT];
        aspects[SimilarityAspect::Spectrum.index()] = Some(0.2);
        aspects[SimilarityAspect::Timbre.index()] = Some(0.9);

        assert_eq!(
            strongest_enabled_aspect(
                &aspects,
                &wavecrate::sample_sources::config::SimilarityAspectSettings::default(),
            ),
            SimilarityAspect::Timbre
        );
    }

    fn color_distance(left: ui::Rgba8, right: ui::Rgba8) -> u16 {
        u16::from(left.r.abs_diff(right.r))
            + u16::from(left.g.abs_diff(right.g))
            + u16::from(left.b.abs_diff(right.b))
    }
}
