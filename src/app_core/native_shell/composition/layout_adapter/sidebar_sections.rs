//! Slotized source/folder section partitioning for the sidebar rows band.

use super::super::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Slot-resolved rectangles for one fixed folder pane inside the sidebar band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarFolderPaneSections {
    pub bounds: Rect,
    pub source_rows: Rect,
    pub header: Rect,
    pub rows: Rect,
}

/// Slot-resolved source/folder section rectangles inside the sidebar rows band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarRowSections {
    pub upper_folder_pane: SidebarFolderPaneSections,
    pub lower_folder_pane: SidebarFolderPaneSections,
}

/// Rendered row-count inputs used for source/folder section partitioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SidebarRowCounts {
    pub source_rows: usize,
    pub upper_tree_rows: usize,
    pub lower_tree_rows: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SidebarPaneHeights {
    source_rows: f32,
    source_gap: f32,
    header: f32,
    rows: f32,
}

/// Compute source/folder sections inside `layout.sidebar_rows`.
pub(crate) fn compute_sidebar_row_sections(
    sidebar_rows: Rect,
    sizing: SizingTokens,
    counts: SidebarRowCounts,
) -> SidebarRowSections {
    let section_bounds = inset_vertical(
        sidebar_rows,
        sizing.panel_section_padding_top,
        sizing.panel_section_padding_bottom,
    );
    let empty = Rect::from_min_max(section_bounds.max, section_bounds.max);
    let midpoint = section_bounds.min.y + (section_bounds.height() * 0.5);
    let upper_bounds = Rect::from_min_max(
        section_bounds.min,
        Point::new(section_bounds.max.x, midpoint.min(section_bounds.max.y)),
    );
    let lower_bounds = Rect::from_min_max(
        Point::new(section_bounds.min.x, midpoint.max(section_bounds.min.y)),
        section_bounds.max,
    );
    SidebarRowSections {
        upper_folder_pane: resolve_pane_sections(
            upper_bounds,
            sizing,
            counts.source_rows,
            counts.upper_tree_rows,
        ),
        lower_folder_pane: resolve_pane_sections(
            lower_bounds,
            sizing,
            counts.source_rows,
            counts.lower_tree_rows,
        ),
    }
    .with_empty_fallback(empty)
}

impl SidebarRowSections {
    fn with_empty_fallback(mut self, empty: Rect) -> Self {
        if self.upper_folder_pane.bounds.height() <= 0.0 {
            self.upper_folder_pane = empty_pane_sections(empty);
        }
        if self.lower_folder_pane.bounds.height() <= 0.0 {
            self.lower_folder_pane = empty_pane_sections(empty);
        }
        self
    }
}

fn empty_pane_sections(empty: Rect) -> SidebarFolderPaneSections {
    SidebarFolderPaneSections {
        bounds: empty,
        source_rows: empty,
        header: empty,
        rows: empty,
    }
}

fn resolve_pane_sections(
    bounds: Rect,
    sizing: SizingTokens,
    source_rows: usize,
    tree_rows: usize,
) -> SidebarFolderPaneSections {
    let heights = resolve_pane_heights(bounds.height(), sizing, source_rows, tree_rows);
    let source_rows_rect = rect_from_top(bounds, bounds.min.y, heights.source_rows);
    let header_top = (source_rows_rect.max.y + heights.source_gap).min(bounds.max.y);
    let header_rect = rect_from_top(bounds, header_top, heights.header);
    let rows_rect = rect_from_top(bounds, header_rect.max.y, heights.rows);
    SidebarFolderPaneSections {
        bounds,
        source_rows: clamp_rect_to_bounds(source_rows_rect, bounds),
        header: clamp_rect_to_bounds(header_rect, bounds),
        rows: clamp_rect_to_bounds(rows_rect, bounds),
    }
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn inset_vertical(rect: Rect, top: f32, bottom: f32) -> Rect {
    let top = top.max(0.0);
    let bottom = bottom.max(0.0);
    let max_inset = (rect.height() * 0.5).max(0.0);
    let top = top.min(max_inset);
    let bottom = bottom.min(max_inset);
    Rect::from_min_max(
        Point::new(rect.min.x, (rect.min.y + top).min(rect.max.y)),
        Point::new(rect.max.x, (rect.max.y - bottom).max(rect.min.y)),
    )
}

fn resolve_pane_heights(
    available_height: f32,
    sizing: SizingTokens,
    source_rows: usize,
    tree_rows: usize,
) -> SidebarPaneHeights {
    let source_layout_rows = source_rows.max(1);
    let source_demand_height = stack_height(
        source_layout_rows,
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    let source_min_rows = source_layout_rows
        .min(sizing.source_rows_min_when_split)
        .max(1);
    let source_min_height = stack_height(
        source_min_rows,
        sizing.source_row_height,
        sizing.source_row_gap,
    );
    let header_height = sizing
        .folder_header_block_height
        .min(available_height.max(0.0));
    let source_gap = if source_layout_rows > 0 {
        sizing.sidebar_section_gap
    } else {
        0.0
    };
    let folder_row_min = compact_tree_rows_height(sizing);
    let reserved_folder_height = header_height + folder_row_min;

    let (source_rows, source_gap) = if source_min_height + source_gap + reserved_folder_height
        <= available_height
    {
        let max_source_height = (available_height - source_gap - reserved_folder_height).max(0.0);
        (source_demand_height.min(max_source_height), source_gap)
    } else if reserved_folder_height <= available_height {
        (0.0, 0.0)
    } else {
        (0.0, 0.0)
    };

    let rows_height = (available_height - source_rows - source_gap - header_height).max(0.0);
    let folder_demand = stack_height(
        tree_rows.max(1),
        sizing.folder_row_height,
        sizing.folder_row_gap,
    );
    SidebarPaneHeights {
        source_rows,
        source_gap,
        header: header_height,
        rows: rows_height.min(folder_demand.max(rows_height)),
    }
}

fn compact_tree_rows_height(sizing: SizingTokens) -> f32 {
    stack_height(1, sizing.folder_row_height, sizing.folder_row_gap)
}

fn rect_from_top(bounds: Rect, top: f32, height: f32) -> Rect {
    let min_y = top.clamp(bounds.min.y, bounds.max.y);
    let max_y = (min_y + height.max(0.0)).min(bounds.max.y);
    Rect::from_min_max(
        Point::new(bounds.min.x, min_y),
        Point::new(bounds.max.x, max_y),
    )
}

fn stack_height(rows: usize, row_height: f32, gap: f32) -> f32 {
    if rows == 0 {
        return 0.0;
    }
    (rows as f32 * row_height.max(0.0)) + ((rows.saturating_sub(1)) as f32 * gap.max(0.0))
}
