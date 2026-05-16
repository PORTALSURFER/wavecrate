//! Source/folder section partitioning for the sidebar rows band.

use super::super::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Resolved rectangles for the visible folder browser inside the sidebar band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarFolderPaneSections {
    pub bounds: Rect,
    pub source_rows: Rect,
    pub header: Rect,
    pub rows: Rect,
}

/// Source/folder section rectangles inside the sidebar rows band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarRowSections {
    pub upper_folder_pane: SidebarFolderPaneSections,
    pub lower_folder_pane: SidebarFolderPaneSections,
}

/// Compact library-workspace bands inside the sidebar rows band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarWorkspaceSections {
    pub sources: Rect,
    pub tags: Rect,
    pub filters: Rect,
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
///
/// The model still carries upper/lower pane fields for persisted-state
/// compatibility, but the native sidebar now exposes one source list and one
/// folder browser. The visible section is returned in the upper slot; callers
/// can remap it to an active compatibility slot when needed.
pub(crate) fn compute_sidebar_row_sections(
    sidebar_rows: Rect,
    sizing: SizingTokens,
    counts: SidebarRowCounts,
) -> SidebarRowSections {
    let workspace = compute_sidebar_workspace_sections(sidebar_rows, sizing);
    let section_bounds = workspace.sources;
    let empty = Rect::from_min_max(section_bounds.max, section_bounds.max);
    SidebarRowSections {
        upper_folder_pane: resolve_pane_sections(
            section_bounds,
            sizing,
            counts.source_rows,
            counts.upper_tree_rows,
        ),
        lower_folder_pane: empty_pane_sections(empty),
    }
    .with_empty_fallback(empty)
}

/// Compute top Sources plus bottom Tags/Filters bands for the compact library workspace.
pub(crate) fn compute_sidebar_workspace_sections(
    sidebar_rows: Rect,
    sizing: SizingTokens,
) -> SidebarWorkspaceSections {
    let bounds = inset_vertical(
        sidebar_rows,
        sizing.panel_section_padding_top,
        sizing.panel_section_padding_bottom,
    );
    let empty = bounds.empty_at_min();
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return SidebarWorkspaceSections {
            sources: empty,
            tags: empty,
            filters: empty,
        };
    }

    let gap = sizing.sidebar_section_gap.max(3.0);
    let row_height = sizing.browser_row_height.max(18.0);
    let label_height = (sizing.font_meta + sizing.text_inset_y).max(10.0);
    let desired_tags_height = label_height + row_height * 2.0 + gap;
    let desired_filters_height = label_height + row_height * 6.0 + gap;
    let bottom_demand = desired_tags_height + desired_filters_height + gap;
    let min_sources_height = (sizing.source_row_height + sizing.folder_row_height + gap)
        .min(bounds.height())
        .max(0.0);
    let available_bottom = (bounds.height() - min_sources_height).max(0.0);
    let bottom_scale = if bottom_demand <= available_bottom || bottom_demand <= 0.0 {
        1.0
    } else {
        (available_bottom / bottom_demand).clamp(0.0, 1.0)
    };
    let tags_height = desired_tags_height * bottom_scale;
    let filters_height = desired_filters_height * bottom_scale;
    let used_bottom = tags_height + filters_height + if bottom_scale > 0.0 { gap } else { 0.0 };
    let sources_bottom = (bounds.max.y - used_bottom).max(bounds.min.y);
    let tags_top = if tags_height > 0.0 {
        (sources_bottom + gap).min(bounds.max.y)
    } else {
        bounds.max.y
    };
    let filters_top = if filters_height > 0.0 {
        (tags_top + tags_height + gap).min(bounds.max.y)
    } else {
        bounds.max.y
    };

    SidebarWorkspaceSections {
        sources: clamp_rect_to_bounds(
            Rect::from_min_max(bounds.min, Point::new(bounds.max.x, sources_bottom)),
            bounds,
        ),
        tags: clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(bounds.min.x, tags_top),
                Point::new(bounds.max.x, (tags_top + tags_height).min(bounds.max.y)),
            ),
            bounds,
        ),
        filters: clamp_rect_to_bounds(
            Rect::from_min_max(
                Point::new(bounds.min.x, filters_top),
                Point::new(
                    bounds.max.x,
                    (filters_top + filters_height).min(bounds.max.y),
                ),
            ),
            bounds,
        ),
    }
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
    rect.clamp_to(bounds)
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
