use std::collections::HashMap;

use super::StarmapItem;

pub(super) const STARMAP_PROJECTION_INDEX_GRID: i32 = 48;

#[derive(Clone, Debug, Default)]
pub(super) struct StarmapProjectionIndex {
    cells: HashMap<StarmapProjectionCell, Vec<usize>>,
    file_indices: HashMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct StarmapProjectionCell {
    x: i32,
    y: i32,
}

impl StarmapProjectionIndex {
    pub(super) fn build(items: &[StarmapItem]) -> Self {
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

    pub(super) fn preview_warm_indices(
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

    #[cfg(test)]
    pub(super) fn occupied_cell_count(&self) -> usize {
        self.cells.len()
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
pub(super) struct StarmapPreviewWarmScan {
    pub(super) indices: Vec<usize>,
    pub(super) inspected_count: usize,
    pub(super) cell_count: usize,
    pub(super) visited_cell_count: usize,
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
